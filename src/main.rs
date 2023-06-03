use std::fs;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::thread;
mod ogn;
use chrono::{NaiveDate, Datelike, Utc};
use ogn::{thread_ogn, creer_chemin_jour};
use serveur::{ajouter_requete, enlever_requete, mettre_a_jour, MiseAJour, Vol};
use simple_http_parser::request;
use std::sync::{Arc, Mutex};

fn main() {
    let requetes_en_cours: Arc<Mutex<Vec<serveur::Client>>> = Arc::new(Mutex::new(Vec::new()));
    let ecouteur = TcpListener::bind("127.0.0.1:7878").unwrap();

    let vols: Arc<Mutex<Vec<Vol>>> = Arc::new(Mutex::new(Vec::new()));
    let vols_lock = vols.lock().unwrap();
    lire_vols_date((*vols_lock).clone(), 2023, 04, 05);
    drop(vols_lock);

    let vols_thread = vols.clone();

    // creation du dossier de travail si besoin
    let mut chemins = fs::read_dir("./").unwrap();
    if !(chemins.any(|chemin| {
        chemin.unwrap().path().to_str().unwrap().to_string() == "./dossier_de_travail"
    })) {
        fs::create_dir(format!("./dossier_de_travail")).unwrap();
    }

    //on spawn le thread qui va s'occuper de ogn
    thread::spawn(move || {
        thread_ogn(vols_thread);
    });

    for flux in ecouteur.incoming() {
        let flux = flux.unwrap();
        let requetes_en_cours = requetes_en_cours.clone();

        let vols = vols.clone();

        thread::spawn(move || {
            gestion_connexion(flux, requetes_en_cours, vols);
        });
    }
}

fn gestion_connexion(
    mut flux: TcpStream,
    requetes_en_cours: Arc<Mutex<Vec<serveur::Client>>>,
    vols: Arc<Mutex<Vec<Vol>>>,
) {
    let adresse = format!("{}", (flux.peer_addr().unwrap()));

    let requetes_en_cours_lock = requetes_en_cours.lock().unwrap();
    ajouter_requete(requetes_en_cours_lock.to_vec(), adresse.clone());
    drop(requetes_en_cours_lock);

    let mut tampon = [0; 16384];
    flux.read(&mut tampon).unwrap();

    let requete_brute = String::from_utf8_lossy(&tampon).to_owned();
    let requete_parse = request::Request::from(&requete_brute).unwrap();
    let chemin = requete_parse.path;
    println!("{}", chemin);
    let corps_json = requete_parse.body;
    let nom_fichier = match chemin.as_str() {
        "/" => "./planche/example.html",
        "/vols" => "vols",
        "/vols.json" => "vols",
        string => string,
    };
    let mut ligne_statut = "HTTP/1.1 200 OK";
    let mut headers = String::new();
    let contenu: String = if (nom_fichier != "vols") && (nom_fichier != "miseajour") {
        if nom_fichier[nom_fichier.len() - 5..nom_fichier.len()].to_string() == ".json".to_string()
        {
            headers.push_str(
                "Content-Type: application/json\
                \nAccess-Control-Allow-Origin: *",
            );
        }
        fs::read_to_string(format!("./parametres{}", nom_fichier)).unwrap_or_else(|_| {
            ligne_statut = "HTTP/1.1 404 NOT FOUND";
            fs::read_to_string("./parametres/planche/404.html").unwrap_or_else(|err| {
                eprintln!("pas de 404.html !! : {}", err);
                "".to_string()
            })
        })
    } else if nom_fichier == "vols" {
        println!("vols");
        let vols_lock = vols.lock().unwrap();
        let vols_vec = (*vols_lock).clone();
        drop(vols_lock);
        let mut vols_str = String::new();
        vols_str.push_str("[\n");
        for vol in vols_vec {
            println!("1");
            vols_str.push_str(vol.to_json().as_str());
            vols_str.push_str(",");
            println!("{}", vol.to_json().as_str());
        }
        vols_str = vols_str[0..(vols_str.len() - 1)].to_string(); // on enleve la virgule de trop
        vols_str.push_str("\n]"); //on ferme
        headers.push_str(
            "Content-Type: application/json\
            \nAccess-Control-Allow-Origin: *",
        );
        println!("{}", vols_str);
        vols_str
    } else if nom_fichier == "miseajour" {
        // les trois champs d'une telle requete sont séparés par des virgules tels que: "4,decollage,12:24,"
        let mut mise_a_jour = MiseAJour::new();
        mise_a_jour
            .parse(json::parse(corps_json.as_str()).unwrap())
            .unwrap();

        let vols_lock = vols.lock().unwrap();
        let vols_vec = (*vols_lock).clone();
        mettre_a_jour(vols_vec, mise_a_jour);
        drop(vols_lock);

        String::from("ok!")
    } else {
        "".to_string()
    };

    let reponse = format!(
        "{}\r\nContent-Length: {}\n\
        {}\r\n\r\n{}",
        ligne_statut,
        contenu.len(),
        headers,
        contenu
    );

    flux.write(reponse.as_bytes()).unwrap();
    flux.flush().unwrap();

    let requetes_en_cours_lock = requetes_en_cours.lock().unwrap();
    enlever_requete(requetes_en_cours_lock.to_vec(), adresse);
    drop(requetes_en_cours_lock);
}

fn lire_vols_chemin(mut vols: Vec<Vol>, chemin_jour: String) {
    let fichiers_vols = fs::read_dir(format!("./dossier_de_travail/{}", chemin_jour)).unwrap();
    
    for vol in fichiers_vols {
        let fichier_vol_str = fs::read_to_string(format!("./dossier_de_travail/{}/{}", chemin_jour, vol.unwrap().path().to_str().unwrap())).unwrap();
        let vol_json_parse = Vol::from_json(json::parse(fichier_vol_str.as_str()).unwrap());
        vols.push(vol_json_parse);
    }
}

fn lire_vols_date(vols: Vec<Vol>, annee: i32, mois: u32, jour: u32) {
    
    creer_chemin_jour(annee.to_string(), mois.to_string(), jour.to_string());
    let chemin = format!("./{}/{}/{}", annee, mois, jour);
    lire_vols_chemin(vols, chemin);
}

fn lire_vols_jour(vols: Vec<Vol>) {
    let date_maintenant = Utc::now();
    let annee = date_maintenant.year();
    let mois = date_maintenant.month();
    let jour = date_maintenant.day();
    lire_vols_date(vols, annee, mois, jour);
}

mod tests;
