use std::fs;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::thread;
mod ogn;
use chrono::{Datelike, Utc};
use ogn::{thread_ogn, creer_chemin_jour};
use serveur::*;
use simple_http_parser::request;
use std::sync::{Arc, Mutex};

fn main() {
    let requetes_en_cours: Arc<Mutex<Vec<Client>>> = Arc::new(Mutex::new(Vec::new()));
    let ecouteur = TcpListener::bind("127.0.0.1:7878").unwrap();

    let vols: Arc<Mutex<Vec<Vol>>> = Arc::new(Mutex::new(Vec::new()));
    let mut vols_lock = vols.lock().unwrap();
    *vols_lock = vols_enregistres_date(2023, 04, 25);
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
    requetes_en_cours: Arc<Mutex<Vec<Client>>>,
    vols: Arc<Mutex<Vec<Vol>>>,
) {
    let adresse = format!("{}", (flux.peer_addr().unwrap()));

    requetes_en_cours.clone().incrementer(adresse.clone());

    let mut tampon = [0; 16384];
    flux.read(&mut tampon).unwrap();

    let requete_brute = String::from_utf8_lossy(&tampon).to_owned();
    let requete_parse = request::Request::from(&requete_brute).unwrap();
    let chemin = requete_parse.path;

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
        //on recupere la liste de vols
        let vols_lock = vols.lock().unwrap();
        let vols_vec = (*vols_lock).clone();
        drop(vols_lock);
        
        let vols_str = vols_vec.vers_json();
        headers.push_str(
            "Content-Type: application/json\
            \nAccess-Control-Allow-Origin: *",
        );
        vols_str
    } else if nom_fichier == "miseajour" {
        // les trois champs d'une telle requete sont séparés par des virgules tels que: "4,decollage,12:24,"
        let mut mise_a_jour = MiseAJour::new();
        mise_a_jour
            .parse(json::parse(corps_json.as_str()).unwrap())
            .unwrap();

        let vols_lock = vols.lock().unwrap();
        let mut vols_vec = (*vols_lock).clone();
        vols_vec.mettre_a_jour(mise_a_jour);
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

    requetes_en_cours.clone().decrementer(adresse);
}

fn vols_enregistres_chemin(chemin_jour: String) -> Vec<Vol>{
    let fichiers_vols = fs::read_dir(format!("./dossier_de_travail/{}", chemin_jour)).unwrap();
    let mut vols: Vec<Vol> = Vec::new();
    
    for vol in fichiers_vols {
        let nom_fichier = vol.unwrap().path().to_str().unwrap().to_string();
        let fichier_vol_str = fs::read_to_string(format!("{}", nom_fichier)).unwrap();
        let vol_json_parse = Vol::depuis_json(json::parse(fichier_vol_str.as_str()).unwrap());
        vols.push(vol_json_parse);
    }
    vols
}

fn vols_enregistres_date(annee: i32, mois: u32, jour: u32) -> Vec<Vol>{
    
    creer_chemin_jour(annee, mois, jour);
    
    let jour_str = nom_fichier_date(jour as i32);
    let mois_str = nom_fichier_date(mois as i32);
    
    let chemin = format!("./{}/{}/{}", annee, mois_str, jour_str);

    vols_enregistres_chemin(chemin)
    
}

fn _vols_enregistres_jour() -> Vec<Vol> {
    let date_maintenant = Utc::now();
    let annee = date_maintenant.year();
    let mois = date_maintenant.month();
    let jour = date_maintenant.day();
    vols_enregistres_date(annee, mois, jour)
}

mod tests;
