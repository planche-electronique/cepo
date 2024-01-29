use std::fs;
use std::sync::{Arc, Mutex};

use brick_ogn::flightlog::update::ObsoleteUpdates;
use brick_ogn::flightlog::update::Update;
use brick_ogn::flightlog::FlightLog;
use serveur::client::{Client, UsageControl};
use serveur::flightlog::Storage;
use serveur::ogn::synchronisation_ogn;
use serveur::{data_dir, Configuration, Context};

use chrono::NaiveDate;

use human_panic::setup_panic;

//hyper utils
use std::convert::Infallible;
use std::net::{IpAddr, SocketAddr};

use hyper::header::*;
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use hyper::{Method, StatusCode};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    //initialisation des outils cli (confy, log, panic)
    let configuration = confy::load("cepo", None).unwrap_or_else(|err| {
        log::warn!(
            "Config flie not ofund, using default : {}",
            err
        );
        Configuration::default()
    });
    confy::store("cepo", None, configuration.clone())?;
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(configuration.niveau_log.clone()),
    )
    .init();

    #[cfg(debug_assertions)]
    setup_panic!();

    log::info!("Starting up...");

    let date_today = chrono::Local::now().date_naive();
    let current_requests: Arc<Mutex<Vec<Client>>> = Arc::new(Mutex::new(Vec::new()));
    //let ecouteur = TcpListener::bind("127.0.0.1:7878").unwrap();
    let adress = SocketAddr::from(([0, 0, 0, 0], configuration.clone().port as u16));
    // creation du dossier de travail si besoin
    if !(crate::data_dir().as_path().exists()) {
        fs::create_dir_all(data_dir().as_path())
            .expect("Could not create data_dir on your platform.");
        log::info!("Create dir for data.");
    }
    dbg!(date_today);
    let flightlog = FlightLog::load(date_today).await.unwrap_or_else(|_| {
        let mut fl = FlightLog::new();
        fl.date = date_today;
        fl
    });
    let flightlog_arc: Arc<Mutex<FlightLog>> = Arc::new(Mutex::new(flightlog));

    let updates_arc: Arc<Mutex<Vec<Update>>> = Arc::new(Mutex::new(Vec::new()));
    let context: Context = Context {
        configuration,
        flightlog: flightlog_arc,
        updates: updates_arc,
        current_requests,
    };
    let context_svc = context.clone();
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
    let f_synchronisation_secs_clone = context
        .clone()
        .configuration
        .clone()
        .f_synchronisation_secs
        .clone() as u64;
    //on spawn le thread qui va s'occuper de ogn
    tokio::spawn(async move {
        log::info!("Launching the OGN thread.");
        loop {
            let res = synchronisation_ogn(&context);
            tokio::time::sleep(tokio::time::Duration::from_secs(
                f_synchronisation_secs_clone,
            ))
            .await; //5 minutes
            res.await.unwrap();
        }
    });
    let server = Server::bind(&adress)
        .serve(service)
        .with_graceful_shutdown(signal_extinction());
    log::info!("Server started.");
    server.await?;
    //drop(context);
    Ok(())
}

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
                let query = parts.uri.query();
                let date = match query {
                    Some(query_str) => {
                        NaiveDate::parse_from_str(query_str, "date=%Y/%m/%d").unwrap_or_else(|err| {
                            log::error!("Could not parse request's date ({query_str}) : {err}");
                            today
                        })
                    }
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
                let mut updates_lock = context.updates.lock().unwrap();
                let majs = (*updates_lock).clone();
                (*updates_lock).remove_obsolete_updates(chrono::Duration::minutes(5));
                drop(updates_lock);
                *response.body_mut() = Body::from(serde_json::to_string(&majs).unwrap_or_default());
            }
            (&Method::GET, "/infos.json") => {
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
                for char in std::str::from_utf8(&corps_str.await?).unwrap().to_string().chars() {
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

async fn signal_extinction() {
    // Attendre pour le signal CTRL+C
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install signal handler for Ctrl-C");
}

mod tests;
