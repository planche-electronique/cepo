use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

use serveur::client::{Client, VariationRequete};
use serveur::ogn::thread_ogn;
use serveur::planche::mise_a_jour::{MiseAJour, MiseAJourJson, MiseAJourObsoletes};
use serveur::planche::{MettreAJour, Planche};
use serveur::vol::{Vol, VolJson};

use chrono::NaiveDate;
use simple_http_parser::request;

//hyper utils
use std::convert::Infallible;
use std::net::SocketAddr;

use hyper::header::*;
use hyper::service::{make_service_fn, service_fn};
use hyper::{header, Method, StatusCode};
use hyper::{Body, Request, Response, Server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::init();

    log::info!("Démarrage...");
    let date_aujourdhui = chrono::Local::now().date_naive();
    let requetes_en_cours: Arc<Mutex<Vec<Client>>> = Arc::new(Mutex::new(Vec::new()));
    //let ecouteur = TcpListener::bind("127.0.0.1:7878").unwrap();
    let adresse = SocketAddr::from(([127, 0, 0, 1], 7878));

    // creation du dossier de travail si besoin
    if !(Path::new("../site/dossier_de_travail").exists()) {
        log::info!("Création du dossier de travail.");
        fs::create_dir("../site/dossier_de_travail").unwrap();
        log::info!("Dossier de travail créé.");
    }

    let planche_arc: Arc<Mutex<Planche>> = Arc::new(Mutex::new(Planche::new()));
    {
        let mut planche_lock = planche_arc.lock().unwrap();
        *planche_lock = Planche::du(date_aujourdhui).await;
    }

    let majs_arc: Arc<Mutex<Vec<MiseAJour>>> = Arc::new(Mutex::new(Vec::new()));

    let planche_thread = planche_arc.clone();

    log::info!("Serveur démarré.");

    // let service = make_service_fn(|_conn| async {
    //     Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| async move {
    //         gestion_connexion(req, requetes_en_cours, planche_arc, majs_arc);
    //     }))
    // });
    
    let service = make_service_fn(|_conn| {
        let requetes_en_cours = requetes_en_cours.clone();
        let planche_arc = planche_arc.clone();
        let majs_arc = majs_arc.clone();
        
        async move { 
            Ok::<_, Infallible>(service_fn(move |req| {
                gestion_connexion(req, requetes_en_cours.clone(), planche_arc.clone(), majs_arc.clone())
            }))
        }
    });

    //on spawn le thread qui va s'occuper de ogn
    let _ = thread::Builder::new()
        .name("Thread OGN".to_string())
        .spawn(move || {
            log::info!("Lancement du thread qui s'occupe des requetes OGN automatiquement.");
            thread_ogn(planche_thread);
        });

    let serveur = Server::bind(&adresse).serve(service);
    serveur.await?;
    Ok(())
}

async fn gestion_connexion(
    req: Request<Body>,
    requetes_en_cours: Arc<Mutex<Vec<Client>>>,
    planche: Arc<Mutex<Planche>>,
    majs_arc: Arc<Mutex<Vec<MiseAJour>>>,
) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    let adresse = req.uri().path().to_string().clone();

    requetes_en_cours.clone().incrementer(adresse.clone());

    let chemin = format!("../site{}", req.uri().path());
    let (parties, body) = req.into_parts();
    let corps_str = std::str::from_utf8(&hyper::body::to_bytes(body).await?)
        .unwrap()
        .to_string();

    log::info!(
        "{} : requete du fichier {}",
        adresse.clone(),
        chemin.clone()
    );

    let mut reponse = Response::new(Body::empty());

    match parties.method {
        Method::GET => {
            if chemin == *"../site/" {
                *reponse.body_mut() = Body::from(fs::read_to_string("../site/index.html").unwrap());
            } else if chemin == *"../site/majs" {
                reponse
                    .headers_mut()
                    .insert(CONTENT_TYPE, "application/json".parse().unwrap());
                reponse.headers_mut().insert(
                    ACCESS_CONTROL_ALLOW_HEADERS,
                    "content-type, origin".parse().unwrap(),
                );
                reponse
                    .headers_mut()
                    .insert(ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
                let mut majs_lock = majs_arc.lock().unwrap();
                let majs = (*majs_lock).clone();
                (*majs_lock).enlever_majs_obsoletes(chrono::Duration::minutes(5));
                drop(majs_lock);
                *reponse.body_mut() = Body::from(majs.vers_json());
            } else if &(chemin[8..12]) == "vols" {
                reponse
                    .headers_mut()
                    .insert(CONTENT_TYPE, "application/json".parse().unwrap());
                reponse.headers_mut().insert(
                    ACCESS_CONTROL_ALLOW_HEADERS,
                    "content-type, origin".parse().unwrap(),
                );
                reponse
                    .headers_mut()
                    .insert(ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
                let date_str = &chemin[12..23];
                let date = NaiveDate::parse_from_str(date_str, "/%Y/%m/%d").unwrap();

                let vols: Vec<Vol> = Vec::du(date);
                *reponse.body_mut() = Body::from(vols.vers_json());

            //fichier de vols "émulé"
            } else if &(chemin[8..15]) == "planche" {
                reponse
                    .headers_mut()
                    .insert(CONTENT_TYPE, "application/json".parse().unwrap());
                reponse.headers_mut().insert(
                    ACCESS_CONTROL_ALLOW_HEADERS,
                    "content-type, origin".parse().unwrap(),
                );
                reponse
                    .headers_mut()
                    .insert(ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
                //on recupere la liste de planche
                let planche_lock = planche.lock().unwrap();
                let clone_planche = (*planche_lock).clone();
                drop(planche_lock);
                *reponse.body_mut() = Body::from(clone_planche.vers_json());

            //fichier de vols "émulé"
            } else if &chemin[8..12] != "vols" {
                if chemin[chemin.len() - 5..chemin.len()] == *".json" {
                    reponse
                        .headers_mut()
                        .insert(CONTENT_TYPE, "application/json".parse().unwrap());
                    reponse
                        .headers_mut()
                        .insert(ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
                } else if chemin[chemin.len() - 3..chemin.len()] == *".js" {
                    reponse
                        .headers_mut()
                        .insert(CONTENT_TYPE, "application/javascript".parse().unwrap());
                    reponse
                        .headers_mut()
                        .insert(ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
                }
                *reponse.body_mut() = Body::from(
                    fs::read_to_string(format!("../site/{}", chemin)).unwrap_or_else(|_| {
                        *reponse.status_mut() = StatusCode::NOT_FOUND;
                        fs::read_to_string("../site/404.html").unwrap_or_else(|err| {
                            log::info!("pas de 404.html !! : {}", err);
                            "".to_string()
                        })
                    }),
                );
            }
        }

        Method::POST => {
            if chemin == "../site/mise_a_jour" {
                // les trois champs d'une telle requete sont séparés par des virgules tels que: "4,decollage,12:24,"
                let mut mise_a_jour = MiseAJour::new();
                let mut corps_json_nettoye = String::new(); //necessite de creer une string qui va contenir
                                                            //seulement les caracteres valies puisque le parser retourne des UTF0000 qui sont invalides pour le parser json
                for char in corps_str.chars() {
                    if char as u32 != 0 {
                        corps_json_nettoye.push_str(char.to_string().as_str());
                    }
                }

                mise_a_jour
                    .parse(json::parse(&corps_json_nettoye).unwrap())
                    .unwrap();
                let date_aujourdhui = chrono::Local::now().date_naive();
                {
                    // On ajoute la mise a jour au vecteur de mises a jour
                    let mut majs_lock = majs_arc.lock().unwrap();
                    (*majs_lock).push(mise_a_jour.clone());
                }

                if mise_a_jour.date != date_aujourdhui {
                    let mut planche_voulue = Planche::du(mise_a_jour.date).await;
                    planche_voulue.mettre_a_jour(mise_a_jour);
                    planche_voulue.enregistrer();
                } else {
                    let mut planche_lock = planche.lock().unwrap();
                    (*planche_lock).mettre_a_jour(mise_a_jour);
                    (*planche_lock).enregistrer();
                    drop(planche_lock);
                }
                reponse
                    .headers_mut()
                    .insert(CONTENT_TYPE, "application/json".parse().unwrap());
                reponse
                    .headers_mut()
                    .insert(ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
            }
        }

        Method::OPTIONS => {
            if chemin == "/mise_a_jour" {
                // les trois champs d'une telle requete sont séparés par des virgules tels que: "4,decollage,12:24,"
                *reponse.status_mut() = StatusCode::NO_CONTENT;
                reponse
                    .headers_mut()
                    .insert(CONNECTION, "keep-alive".parse().unwrap());
                reponse
                    .headers_mut()
                    .insert(ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
                reponse
                    .headers_mut()
                    .insert(ACCESS_CONTROL_MAX_AGE, "86400".parse().unwrap());
                reponse.headers_mut().insert(
                    ACCESS_CONTROL_ALLOW_METHODS,
                    "POST, OPTIONS".parse().unwrap(),
                );
                reponse.headers_mut().insert(
                    ACCESS_CONTROL_ALLOW_HEADERS,
                    "origin, content-type".parse().unwrap(),
                );
            }
        }
        _ => {
            log::error!("Methode non supportée");
        }
    };

    requetes_en_cours.clone().decrementer(adresse);
    Ok(reponse)
}

async fn hello(_: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new(Body::from("Hello World!")))
}
mod tests;
