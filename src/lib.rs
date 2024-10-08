#![warn(missing_docs)]

//! Easy and fast usage of OGN data to load and save takeoffs and landing of glider flights.
//! The program reads under `$XDG_DAT_DIR/cepo/infos.json` to get a list of pilots,
//! names, immatriculations to look at, takeoff_machines and pilots etc.

use crate::client::Client;
use configuration::{Configuration, DayMonitor};
use ogn::synchronisation_ogn;
use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};

use brick_ogn::flightlog::update::Update;
use brick_ogn::flightlog::FlightLog;
use flightlog::Storage;

use hyper::header::*;
use hyper::service::{make_service_fn, service_fn};

pub mod client;
pub mod configuration;
pub mod flight;
pub mod flightlog;
pub mod ogn;

use crate::client::UsageControl;
use brick_ogn::flightlog::update::ObsoleteUpdates;

use chrono::NaiveDate;

#[cfg(not(debug_assertions))]
use human_panic::setup_panic;

//hyper utils
use std::convert::Infallible;
use std::net::{IpAddr, SocketAddr};

use hyper::server::conn::AddrStream;
use hyper::{Body, Request, Response, Server};
use hyper::{Method, StatusCode};

/// Aircraft struct, used to parse OGN API.
pub struct Aircraft {
    /// The type of the aircraft, coming from OGN.
    pub modele: String,
    /// Aircraft category (airplane, glider...) using OGN codes from
    /// [there](https://gitlab.com/davischappins/ogn-flightbook/-/blob/master/doc/API.md.)
    pub category: u8,
    /// The string of the immatriculation e(ex: `F-CMOI`).
    pub immatriculation: String,
}

/// Return a  Two char long string of the number.
/// # Example
/// ```
/// use serveur::{nb_2digits_string};
/// assert_eq!(nb_2digits_string(2), String::from("02"));
/// assert_eq!(nb_2digits_string(20), String::from("20"));
/// ```
pub fn nb_2digits_string(nombre: i32) -> String {
    if nombre > 9 {
        nombre.to_string()
    } else {
        format!("0{}", nombre)
    }
}

/// Create the path associated with a day of the time at "$XDG_DATA_DIR/cepo/year/month/day".
pub fn create_fs_path_day(annee: i32, mois: u32, jour: u32) {
    let jour_str = nb_2digits_string(jour as i32);
    let mois_str = nb_2digits_string(mois as i32);

    let mut path = crate::data_dir();
    path.push(annee.to_string());
    path.push(&mois_str);
    path.push(&jour_str);

    if !path.as_path().exists() {
        dbg!(&path);
        fs::create_dir_all(&path).unwrap();
        log::info!("Creating path {}/{}/{}", annee, &mois_str, &jour_str);
    }
}

/// Server context. Stores configuration, current requests, updates made in the
/// after the last OGN request and the FlightLog of the day.
#[derive(Clone)]
pub struct Context {
    /// Server config.
    pub configuration: Configuration,
    /// The  flightlogs of the day.
    pub flightlogs: HashMap<String, Arc<Mutex<FlightLog>>>,
    /// An vector of Update to keep in memory the updates that were recently made
    /// (after the last OGN automatic request) to avoid to reload the entire
    /// flightlog.
    pub updates: Arc<Mutex<Vec<Update>>>,
    /// A vector that stores who is actually requesting, to limit the number of
    /// concurrent request of the same user. (Some sort of ddos protection).
    pub current_requests: Arc<Mutex<Vec<Client>>>,
}

impl Context {
    /// Context constructor
    pub async fn new(configuration: Configuration) -> Self {
        let current_requests: Arc<Mutex<Vec<Client>>> = Arc::new(Mutex::new(Vec::new()));
        //let ecouteur = TcpListener::bind("127.0.0.1:7878").unwrap();
        // Creation of the working dir if needed
        if !(crate::data_dir().as_path().exists()) {
            fs::create_dir_all(data_dir().as_path())
                .expect("Could not create data_dir on your platform.");
            log::info!("Create dir for data.");
        }
        let flightlogs = (&configuration).create_needed_flightlog_hashmap();

        let updates_arc: Arc<Mutex<Vec<Update>>> = Arc::new(Mutex::new(Vec::new()));
        return Self {
            configuration: configuration.clone(),
            flightlogs: flightlogs.await,
            updates: updates_arc,
            current_requests,
        };
    }
    /// The main server function that is launched after the parsing of the
    /// configuration.
    pub async fn server(&self) -> Result<(), hyper::Error> {
        log::info!("Starting up...");
        let address = SocketAddr::from(([0, 0, 0, 0], self.configuration.port as u16));

        let context_svc = self.clone();
        let service = make_service_fn(|conn: &AddrStream| {
            let context_clone = context_svc.clone();
            let remote_addr = conn.remote_addr().ip().clone();
            async move {
                let context_clone = context_clone.clone();
                let remote_addr = remote_addr.clone();
                Ok::<_, Infallible>(service_fn(move |req| {
                    connection_handler(req, context_clone.clone(), remote_addr)
                }))
            }
        });
        let f_synchronisation_secs_clone = self
            .clone()
            .configuration
            .clone()
            .f_synchronisation_secs
            .clone() as u64;
        // Spawning the regularly requesting OGN thread
        for ap in &self.configuration.airports_configs {
            if ap.day_monitor() == DayMonitor::Always {
                let oaci = ap.oaci();
                let flightlog_arc = self.flightlogs[&oaci].clone();
                let context_c = context_svc.clone();
                tokio::spawn(async move {
                    let flightlog_arc_c = flightlog_arc.clone();
                    log::info!("Launching the OGN thread of {}", &oaci);
                    loop {
                        let res = synchronisation_ogn(flightlog_arc_c.clone(), &oaci, &context_c);

                        tokio::time::sleep(tokio::time::Duration::from_secs(
                            f_synchronisation_secs_clone,
                        ))
                        .await; //5 minutes
                        res.await.unwrap();
                    }
                });
            }
        }
        let server = Server::bind(&address)
            .serve(service)
            .with_graceful_shutdown(signal_extinction());
        log::info!("Server started.");
        server.await?;
        Ok(())
    }
}

/// Handles the parameters for a flightlog get request
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct GetFlightLogsQueryParameters {
    date: NaiveDate,
    oaci: String,
}

/// Handles the parameters for an airports's infos GET request
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct GetInfosQueryParameters {
    oaci: String,
}

/// Handles the parameters for a updates post request
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct PostUpdateQueryParameters {
    oaci: String,
}

/// Main connexion handler for hyper server
async fn connection_handler(
    req: Request<Body>,
    context: Context,
    remote_addr: IpAddr,
) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    let today = chrono::Local::now().date_naive();

    if context
        .current_requests
        .clone()
        .increase_usage(&remote_addr)
    {
        let (parts, body) = req.into_parts();

        let mut full_path_b = dirs::data_dir().expect(
            "Could not deduce where to store \
        files. Check your platform compatibility with dirs \
        (https://crates.io/crates/dirs) crate.",
        );
        full_path_b.push(parts.uri.path());

        let corps_str = hyper::body::to_bytes(body);

        log::info!(
            "Request of file {} {}",
            &full_path_b
                .to_str()
                .expect("Path to standard storage is not valid UTF-8 !"),
            &parts.uri.query().unwrap_or_default()
        );

        let mut response = Response::new(Body::empty());

        match (&parts.method, parts.uri.path()) {
            (&Method::GET, "/flightlog") => {
                add_get_headers(&mut response);
                let query = parts.uri.query().unwrap();
                let query_parameters: GetFlightLogsQueryParameters = serde_qs::from_str(query)
                    .unwrap_or_else(|err| {
                        log::error!("Error while deserializing query objects: {err}");
                        context
                            .current_requests
                            .clone()
                            .decrease_usage(&remote_addr);
                        panic!();
                    });
                if query_parameters.date == today {
                    let flightlog_lock = context.flightlogs[&query_parameters.oaci].lock().unwrap();
                    let clone_planche = (*flightlog_lock).clone();
                    drop(flightlog_lock);
                    *response.body_mut() =
                        Body::from(serde_json::to_string(&clone_planche).unwrap_or_default());
                } else {
                    *response.body_mut() = Body::from(
                        serde_json::to_string(
                            &FlightLog::from_day(
                                query_parameters.date,
                                &query_parameters.oaci,
                                &context,
                            )
                            .await
                            .unwrap(),
                        )
                        .unwrap_or_else(|err| {
                            log::error!(
                                "Could not load FlightLog either from disk or network ! : {err}"
                            );
                            let mut fl = FlightLog::default();
                            fl.date = today;
                            serde_json::to_string(&fl).unwrap()
                        }),
                    );
                }
            }
            (&Method::GET, "/updates") => {
                add_get_headers(&mut response);
                let mut updates_lock = context.updates.lock().unwrap();
                let majs = (*updates_lock).clone();
                (*updates_lock).remove_obsolete_updates(chrono::Duration::minutes(5));
                drop(updates_lock);
                *response.body_mut() = Body::from(serde_json::to_string(&majs).unwrap_or_default());
            }
            (&Method::GET, "/infos") => {
                add_get_headers(&mut response);
                let query = parts.uri.query().unwrap();
                let query_parameters: GetInfosQueryParameters = serde_qs::from_str(query)
                    .unwrap_or_else(|err| {
                        log::error!("Error while deserializing query objects: {err}");
                        context
                            .current_requests
                            .clone()
                            .decrease_usage(&remote_addr);
                        panic!();
                    });
                let infos = context.configuration.infos(&query_parameters.oaci);
                let body = serde_json::to_string(&infos);
                match body {
                    Ok(txt) => {
                        log::info!("Sending infos about {}", query_parameters.oaci);
                        *response.body_mut() = Body::from(txt);
                    }
                    Err(_) => {
                        let err_msg = format!(
                            "Could not find informations about {}. \
                        Please check if the server is configured for this \
                        airport and if you used the correct syntax.",
                            query_parameters.oaci
                        );
                        log::warn!("{}", err_msg);
                        *response.body_mut() = Body::from(err_msg);
                    }
                }
            }
            (&Method::POST, "/updates") => {
                let query = parts.uri.query().unwrap();
                let query_parameters: PostUpdateQueryParameters =
                    serde_qs::from_str(query).unwrap();
                let mut clean_json = String::new();
                for char in std::str::from_utf8(&corps_str.await?)
                    .unwrap()
                    .to_string()
                    .chars()
                {
                    if char as u32 != 0 {
                        clean_json.push_str(char.to_string().as_str());
                    }
                }

                let update: Update = serde_json::from_str(&clean_json).unwrap_or_default();
                {
                    let mut updates_lock = context.updates.lock().unwrap();
                    (*updates_lock).push(update.clone());
                }

                if update.date != today {
                    let mut wanted_flightlog =
                        FlightLog::from_day(update.date, &query_parameters.oaci, &context).await?;
                    wanted_flightlog.update(update);
                    wanted_flightlog.save(&query_parameters.oaci).await;
                } else {
                    let mut flightlog_lock =
                        context.flightlogs[&query_parameters.oaci].lock().unwrap();
                    (*flightlog_lock).update(update);
                    let _ = (*flightlog_lock).save(&query_parameters.oaci);
                    drop(flightlog_lock);
                }
                response
                    .headers_mut()
                    .insert(CONTENT_TYPE, "application/json".parse().unwrap());
                response
                    .headers_mut()
                    .insert(ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
            }
            (&Method::OPTIONS, "/majs") => {
                *response.status_mut() = StatusCode::NO_CONTENT;
                response
                    .headers_mut()
                    .insert(CONNECTION, "keep-alive".parse().unwrap());
                response
                    .headers_mut()
                    .insert(ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
                response
                    .headers_mut()
                    .insert(ACCESS_CONTROL_MAX_AGE, "86400".parse().unwrap());
                response.headers_mut().insert(
                    ACCESS_CONTROL_ALLOW_METHODS,
                    "POST, OPTIONS".parse().unwrap(),
                );
                response.headers_mut().insert(
                    ACCESS_CONTROL_ALLOW_HEADERS,
                    "origin, content-type".parse().unwrap(),
                );
            }
            (&Method::OPTIONS, "/flightlog") => {
                log::info!("Serving OPTIONS for flightlog");
                *response.status_mut() = StatusCode::NO_CONTENT;
                response
                    .headers_mut()
                    .insert(ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
                response.headers_mut().insert(
                    ACCESS_CONTROL_ALLOW_METHODS,
                    "OPTIONS, GET".parse().unwrap(),
                );
            }
            _ => {
                log::error!(
                    "Method or path not available : {:?}; {:?}; {:?}",
                    &parts.method,
                    &parts.uri.path(),
                    &parts.uri.query()
                );
                *response.status_mut() = hyper::StatusCode::NOT_FOUND;
                *response.body_mut() = Body::from(
                    fs::read_to_string(data_dir().as_path().join("404.html")).unwrap_or_else(
                        |err| {
                            log::warn!(
                                "Could not load 404.html : {} Please add it to $XDG_DATA_DIR/cepo.",
                                err
                            );
                            "".to_string()
                        },
                    ),
                );
            }
        };

        context
            .current_requests
            .clone()
            .decrease_usage(&remote_addr);
        Ok(response)
    } else {
        Err("Too many request from this user".into())
    }
}

/// The handler for the end of the program
async fn signal_extinction() {
    // Waiting for the CTRL-C signal
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install signal handler for Ctrl-C");
}

/// A function that provides the basic path for storage using dirs crate to
/// provide platform specific paths.
pub fn data_dir() -> std::path::PathBuf {
    let mut data_dir = dirs::data_dir().expect(
        "Couldn't guess where to store files. Check your os compatibility \
            with dirs (https://crates.io/crates/dirs) crate.",
    );
    data_dir.push("cepo");
    data_dir
}

/// Add common headers to a get Response
pub fn add_get_headers(response: &mut Response<Body>) {
    response
        .headers_mut()
        .insert(CONTENT_TYPE, "application/json".parse().unwrap());
    response.headers_mut().insert(
        ACCESS_CONTROL_ALLOW_HEADERS,
        "content-type, origin".parse().unwrap(),
    );
    response
        .headers_mut()
        .insert(ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
}

#[cfg(test)]
mod tests {
    use crate::GetFlightLogsQueryParameters;
    use chrono::NaiveDate;

    #[test]
    fn get_flightlogs_query_parameters_deser() {
        let query = "date=2020-10-09&oaci=LFLE";
        let str: GetFlightLogsQueryParameters = GetFlightLogsQueryParameters {
            date: NaiveDate::from_ymd_opt(2020, 10, 9).unwrap(),
            oaci: String::from("LFLE"),
        };
        assert_eq!(str, serde_qs::from_str(query).unwrap())
    }
}
