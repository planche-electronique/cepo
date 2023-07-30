use std::fs;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
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

fn main() {
    env_logger::init();
    log::info!("Démarrage...");
    let date_aujourdhui = chrono::Local::now().date_naive();
    let requetes_en_cours: Arc<Mutex<Vec<Client>>> = Arc::new(Mutex::new(Vec::new()));
    let ecouteur = TcpListener::bind("127.0.0.1:7878").unwrap();

    // creation du dossier de travail si besoin
    if !(Path::new("../site/dossier_de_travail").exists()) {
        log::info!("Création du dossier de travail.");
        fs::create_dir("../site/dossier_de_travail").unwrap();
        log::info!("Dossier de travail créé.");
    }

    let planche_arc: Arc<Mutex<Planche>> = Arc::new(Mutex::new(Planche::new()));
    let mut planche_lock = planche_arc.lock().unwrap();
    *planche_lock = Planche::du(date_aujourdhui);
    drop(planche_lock);

    let majs_arc: Arc<Mutex<Vec<MiseAJour>>> = Arc::new(Mutex::new(Vec::new()));

    let planche_thread = planche_arc.clone();

    log::info!("Serveur démarré.");

    //on spawn le thread qui va s'occuper de ogn
    let _ = thread::Builder::new()
        .name("Thread OGN".to_string())
        .spawn(move || {
            log::info!("Lancement du thread qui s'occupe des requetes OGN automatiquement.");
            thread_ogn(planche_thread);
        });

    for flux in ecouteur.incoming() {
        let flux = flux.unwrap();
        let requetes_en_cours = requetes_en_cours.clone();

        let planche_arc = planche_arc.clone();
        let majs_arc = majs_arc.clone();

        let _ = thread::Builder::new()
            .name("Gestion".to_string())
            .spawn(move || {
                gestion_connexion(flux, requetes_en_cours, planche_arc, majs_arc);
            });
    }
}

fn gestion_connexion(
    mut flux: TcpStream,
    requetes_en_cours: Arc<Mutex<Vec<Client>>>,
    planche: Arc<Mutex<Planche>>,
    majs_arc: Arc<Mutex<Vec<MiseAJour>>>,
) {
    let adresse = format!("{}", (flux.peer_addr().unwrap()));

    requetes_en_cours.clone().incrementer(adresse.clone());

    let mut tampon = [0; 16384];
    flux.read_exact(&mut tampon).unwrap();

    let requete_brute = String::from_utf8_lossy(&tampon).into_owned();
    let requete_parse = request::Request::from(&requete_brute)
        .expect("La requête n'a pas pu être parsé correctement.");
    let chemin = requete_parse.path;
    let corps_json = requete_parse.body.clone();
    let mut nom_fichier = chemin.to_string();

    if nom_fichier == *"/" {
        nom_fichier = String::from("/index.html");
    }
    nom_fichier.insert_str(0, "../site");
    log::info!(
        "{} : requete du fichier {}",
        adresse.clone(),
        nom_fichier.clone()
    );

    let mut ligne_statut = "HTTP/1.1 200 OK";
    let mut headers = String::new();

    let contenu: String = match requete_parse.method {
        request::HTTPMethod::GET => {
            //fichier de majs
            if nom_fichier == *"../site/majs" {
                headers.push_str(
                    "Content-Type: application/json\
                    \nAccess-Control-Allow-Headers: origin, content-type\
                    \nAccess-Control-Allow-Origin: *",
                );
                let mut majs_lock = majs_arc.lock().unwrap();
                let majs = (*majs_lock).clone();
                (*majs_lock).enlever_majs_obsoletes(chrono::Duration::minutes(5));
                drop(majs_lock);
                majs.vers_json()
            // fichier de vols "émulé"
            } else if &(nom_fichier[8..12]) == "vols" {
                headers.push_str(
                    "Content-Type: application/json\
                    \nAccess-Control-Allow-Headers: origin, content-type\
                    \nAccess-Control-Allow-Origin: *",
                );
                let date_str = &nom_fichier[12..23];
                let date = NaiveDate::parse_from_str(date_str, "/%Y/%m/%d").unwrap();

                let vols: Vec<Vol> = Vec::du(date);
                vols.vers_json()

            //fichier de vols "émulé"
            } else if &(nom_fichier[8..15]) == "planche" {
                headers.push_str(
                    "Content-Type: application/json\
                    \nAccess-Control-Allow-Headers: origin, content-type\
                    \nAccess-Control-Allow-Origin: *",
                );

                //on recupere la liste de planche
                let planche_lock = planche.lock().unwrap();
                let clone_planche = (*planche_lock).clone();
                drop(planche_lock);
                clone_planche.vers_json()

            //fichier de vols "émulé"
            } else if &nom_fichier[8..12] != "vols" {
                if nom_fichier[nom_fichier.len() - 5..nom_fichier.len()] == *".json" {
                    headers.push_str(
                        "Content-Type: application/json\
                        \nAccess-Control-Allow-Origin: *",
                    );
                } else if nom_fichier[nom_fichier.len() - 3..nom_fichier.len()] == *".js" {
                    headers.push_str(
                        "Content-Type: application/javascript\
                        \nAccess-Control-Allow-Origin: *",
                    );
                }
                fs::read_to_string(format!("../site/{}", nom_fichier)).unwrap_or_else(|_| {
                    ligne_statut = "HTTP/1.1 404 NOT FOUND";
                    fs::read_to_string("../site/404.html").unwrap_or_else(|err| {
                        log::info!("pas de 404.html !! : {}", err);
                        "".to_string()
                    })
                })
            } else {
                "".to_string()
            }
        }

        request::HTTPMethod::POST => {
            if nom_fichier == "../site/mise_a_jour" {
                // les trois champs d'une telle requete sont séparés par des virgules tels que: "4,decollage,12:24,"
                let mut mise_a_jour = MiseAJour::new();
                let mut corps_json_nettoye = String::new(); //necessite de creer une string qui va contenir
                                                            //seulement les caracteres valies puisque le parser retourne des UTF0000 qui sont invalides pour le parser json
                for char in corps_json.chars() {
                    if char as u32 != 0 {
                        corps_json_nettoye.push_str(char.to_string().as_str());
                    }
                }

                mise_a_jour
                    .parse(json::parse(&corps_json_nettoye).unwrap())
                    .unwrap();
                let date_aujourdhui = chrono::Local::now().date_naive();
                // On ajoute la mise a jour au vecteur de mises a jour
                let mut majs_lock = majs_arc.lock().unwrap();
                (*majs_lock).push(mise_a_jour.clone());
                drop(majs_lock);

                if mise_a_jour.date != date_aujourdhui {
                    let mut planche_voulue = Planche::du(mise_a_jour.date);
                    planche_voulue.mettre_a_jour(mise_a_jour);
                    planche_voulue.enregistrer();
                } else {
                    let mut planche_lock = planche.lock().unwrap();
                    (*planche_lock).mettre_a_jour(mise_a_jour);
                    (*planche_lock).enregistrer();
                    drop(planche_lock);
                }

                ligne_statut = "HTTP/1.1 201 Created";

                headers.push_str(
                    "Content-Type: application/json\
                    \nAccess-Control-Allow-Origin: *",
                );

                String::from("")
            } else {
                String::from("")
            }
        }

        request::HTTPMethod::OPTIONS => {
            if nom_fichier == "/mise_a_jour" {
                // les trois champs d'une telle requete sont séparés par des virgules tels que: "4,decollage,12:24,"
                ligne_statut = "HTTP/1.1 204 No Content";

                headers.push_str(
                    "Connection: keep-alive\
                    \nAccess-Control-Allow-Origin: *\
                    \nAccess-Control-Max-Age: 86400\
                    \nAccess-Control-Allow-Methods: POST, OPTIONS\
                    \nAccess-Control-Allow-headers: origin,  content-type",
                );

                String::from("")
            } else {
                String::from("")
            }
        }
        _ => String::from(""),
    };

    let reponse = format!(
        "{}\r\nContent-Length: {}\n\
        {}\r\n\r\n{}",
        ligne_statut,
        contenu.len(),
        headers,
        contenu
    );

    flux.write_all(reponse.as_bytes()).unwrap();
    flux.flush().unwrap();

    requetes_en_cours.clone().decrementer(adresse);
}

mod tests;
