use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

use serveur::client::{Client, VariationRequete};
use serveur::ogn::synchronisation_ogn;
use serveur::planche::mise_a_jour::{MiseAJour, MiseAJourJson, MiseAJourObsoletes};
use serveur::planche::{MettreAJour, Planche};
use serveur::vol::{ChargementVols, Vol, VolJson};
use serveur::{ActifServeur, Configuration};

use chrono::NaiveDate;

use human_panic::setup_panic;

//hyper utils
use std::convert::Infallible;
use std::net::SocketAddr;

use hyper::header::*;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use hyper::{Method, StatusCode};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    //initialisation des outils cli (confy, log, panic)
    let configuration = confy::load("serveur", None).unwrap_or_else(|err| {
        log::warn!(
            "Fichier de configuration non trouvé, utilisation de défaut : {}",
            err
        );
        Configuration::default()
    });
    confy::store("serveur", None, configuration.clone())?;
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(configuration.niveau_log.clone()),
    )
    .init();
    setup_panic!();
    log::info!("Démarrage...");

    let date_aujourdhui = chrono::Local::now().date_naive();
    let requetes_en_cours: Arc<Mutex<Vec<Client>>> = Arc::new(Mutex::new(Vec::new()));
    //let ecouteur = TcpListener::bind("127.0.0.1:7878").unwrap();
    let adresse = SocketAddr::from(([127, 0, 0, 1], configuration.clone().port as u16));

    // creation du dossier de travail si besoin
    if !(Path::new("../site/dossier_de_travail").exists()) {
        log::info!("Création du dossier de travail.");
        fs::create_dir("../site/dossier_de_travail").unwrap();
        log::info!("Dossier de travail créé.");
    }

    let planche_arc: Arc<Mutex<Planche>> = Arc::new(Mutex::new(Planche::new()));
    let planche = Planche::depuis_disque(date_aujourdhui).unwrap();
    {
        let mut planche_lock = planche_arc.lock().unwrap();
        *planche_lock = planche;
        drop(planche_lock);
    }

    let majs_arc: Arc<Mutex<Vec<MiseAJour>>> = Arc::new(Mutex::new(Vec::new()));
    let actif_serveur: ActifServeur = ActifServeur {
        configuration,
        planche: planche_arc,
        majs: majs_arc,
        requetes_en_cours,
    };
    
	let actif_serveur_clone = actif_serveur.clone();
    let service = make_service_fn(|_conn| {
        let actif_serveur = actif_serveur_clone.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                gestion_connexion(
                    req,
                    actif_serveur.clone(),
                )
            }))
        }
    });
    //on spawn le thread qui va s'occuper de ogn
    tokio::spawn(async move {
        log::info!("Lancement du thread qui s'occupe des requetes OGN automatiquement.");
        loop {
            synchronisation_ogn(&actif_serveur).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_secs(
                actif_serveur.clone().configuration.clone().f_synchronisation_secs as u64,
            ))
            .await; //5 minutes
        }
    });
    let serveur = Server::bind(&adresse)
        .serve(service)
        .with_graceful_shutdown(signal_extinction());
    log::info!("Serveur démarré.");
    serveur.await?;
    //drop(actif_serveur);
    Ok(())
}

async fn gestion_connexion(
    req: Request<Body>,
    actif_serveur: ActifServeur,
) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    let adresse = req.uri().path().to_string().clone();

    actif_serveur.requetes_en_cours.clone().incrementer(adresse.clone());

    let chemin = format!("../site{}", req.uri().path());
    let (parties, body) = req.into_parts();
    let corps_str = std::str::from_utf8(&hyper::body::to_bytes(body).await?)
        .unwrap()
        .to_string();

    log::info!("Requete du fichier {}", chemin.clone());

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
                let mut majs_lock = actif_serveur.majs.lock().unwrap();
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

                let vols: Vec<Vol> = Vec::du(date, &actif_serveur).await?;
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
                let planche_lock = actif_serveur.planche.lock().unwrap();
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
                    let mut majs_lock = actif_serveur.majs.lock().unwrap();
                    (*majs_lock).push(mise_a_jour.clone());
                }

                if mise_a_jour.date != date_aujourdhui {
                    let mut planche_voulue = Planche::du(mise_a_jour.date, &actif_serveur).await?;
                    planche_voulue.mettre_a_jour(mise_a_jour);
                    planche_voulue.enregistrer();
                } else {
                    let mut planche_lock = actif_serveur.planche.lock().unwrap();
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

    // actif_serveur.requetes_en_cours.clone().decrementer(adresse);
    Ok(reponse)
}

async fn signal_extinction() {
    // Attendre pour le signal CTRL+C
    tokio::signal::ctrl_c()
        .await
        .expect("Echec a l'installation du gestionnaire de signal Ctrl-C");
}

mod tests;
