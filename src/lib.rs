#![warn(missing_docs)]

//! Easy and fast usage of OGN data to load and save takeoffs and landing of glider flights.
//! The program reads under `$XDG_DAT_DIR/cepo/infos.json` to get a list of pilots,
//! names, immatriculations to look at, takeoff_machines and pilots etc.

use crate::client::Client;
use configuration::Configuration;
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
use crate::ogn::synchronisation_ogn;
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
        log::info!("Création du chemin {}/{}/{}", annee, &mois_str, &jour_str);
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
        // creation du dossier de travail si besoin
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
        //on spawn le thread qui va s'occuper de ogn
        let thread_config = self.clone();
        tokio::spawn(async move {
            log::info!("Launching the OGN thread.");
            loop {
                let res = synchronisation_ogn(&thread_config);
                tokio::time::sleep(tokio::time::Duration::from_secs(
                    f_synchronisation_secs_clone,
                ))
                .await; //5 minutes
                res.await.unwrap();
            }
        });
        let server = Server::bind(&address)
            .serve(service)
            .with_graceful_shutdown(signal_extinction());
        log::info!("Server started.");
        server.await?;
        //drop(context);
        Ok(())
    }
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
                let query = parts.uri.query();
                let date = match query {
                    Some(query_str) => NaiveDate::parse_from_str(query_str, "date=%Y/%m/%d")
                        .unwrap_or_else(|err| {
                            log::error!("Could not parse request's date ({query_str}) : {err}");
                            today
                        }),
                    None => today,
                };
                if date == today {
                    //on recupere la liste de planche
                    let flightlog_lock = context.flightlog.lock().unwrap();
                    let clone_planche = (*flightlog_lock).clone();
                    drop(flightlog_lock);
                    *response.body_mut() =
                        Body::from(serde_json::to_string(&clone_planche).unwrap_or_default());
                } else {
                    *response.body_mut() = Body::from(
                        serde_json::to_string(&FlightLog::from_day(date, &context).await.unwrap())
                            .unwrap_or_else(|err| {
                                log::error!("Could not load FlightLog either from disk or network ! : {err}");
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
            (&Method::GET, "/infos.json") => {
                add_get_headers(&mut response);
                let path = data_dir()
                    .as_path()
                    .join(std::path::Path::new("infos.json"));
                *response.body_mut() = Body::from(fs::read_to_string(path).unwrap_or_else(|err| {
                    log::warn!("Could not load infos.json : {}", err);
                    *response.status_mut() = hyper::StatusCode::NOT_FOUND;
                    "{}".to_string()
                }));
            }
            (&Method::POST, "/majs") => {
                // les trois champs d'une telle requete sont séparés par des virgules tels que: "4,decollage,12:24,"
                let mut clean_json = String::new(); //necessite de creer une string qui va contenir
                                                    //seulement les caracteres valies puisque le parser retourne des UTF0000 qui sont invalides pour le parser json
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
                    // On ajoute la mise a jour au vecteur de mises a jour
                    let mut updates_lock = context.updates.lock().unwrap();
                    (*updates_lock).push(update.clone());
                }

                if update.date != today {
                    let mut wanted_flightlog = FlightLog::from_day(update.date, &context).await?;
                    wanted_flightlog.update(update);
                    wanted_flightlog.save().await;
                } else {
                    let mut flightlog_lock = context.flightlog.lock().unwrap();
                    (*flightlog_lock).update(update);
                    let _ = (*flightlog_lock).save();
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
                // les trois champs d'une telle requete sont séparés par des virgules tels que: "4,decollage,12:24,"
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
    // Attendre pour le signal CTRL+C
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
