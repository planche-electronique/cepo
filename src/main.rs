use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::fs;
use std::thread;
mod ogn;
use ogn::thread_ogn;
use serveur::{ajouter_requete, enlever_requete, Vol};
use std::sync::{Arc, Mutex};
use httparse;



fn main() {

    let requetes_en_cours: Arc<Mutex<Vec<serveur::Client>>> = Arc::new(Mutex::new(Vec::new()));
    let ecouteur = TcpListener::bind("127.0.0.1:7878").unwrap();

    let vols: Arc<Mutex<Vec<Vol>>> = Arc::new(Mutex::new(Vec::new()));

    let vols_thread = vols.clone();
    
    // creation du dossier de travail si besoin
    let mut chemins = fs::read_dir("./").unwrap();
    if !(chemins.any(|chemin| chemin.unwrap().path().to_str().unwrap().to_string() == "./dossier_de_travail")) {
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
    vols: Arc<Mutex<Vec<Vol>>>
) {
    let adresse = format!("{}", (flux.peer_addr().unwrap()));

    let requetes_en_cours_lock = requetes_en_cours.lock().unwrap();
    ajouter_requete(requetes_en_cours_lock.to_vec(), adresse.clone());
    drop(requetes_en_cours_lock);

    let mut tampon = [0; 1024];

    flux.read(&mut tampon).unwrap();
    let mut header = [httparse::EMPTY_HEADER; 1024];
    let mut propriete_requete = httparse::Request::new(&mut header);
    let _requetes_parse = propriete_requete.parse(&tampon).unwrap();

    let nom_fichier = match propriete_requete.path.unwrap_or_default() {
        "/" => "./planche/example.html",
        "/vols" => "vols",
        "/vols.json" => "vols",
        string => string
    };
    let mut ligne_statut = "HTTP/1.1 200 OK";
    let contenu: String = if nom_fichier != "vols" {

        fs::read_to_string(format!("./parametres{}", nom_fichier)).unwrap_or_else(|_| {
            ligne_statut = "HTTP/1.1 404 NOT FOUND";
            fs::read_to_string("./parametres/planche/404.html").unwrap_or_else(|err| {
                eprintln!("pas de 404.html !! : {}", err);
                "".to_string()
            })
        })
    } else {
        let vols_lock = vols.lock().unwrap();
        let vols_vec = (*vols_lock).clone();
        drop(vols_lock);
        let mut vols_str = String::new();
        vols_str.push_str("[");
        for vol in vols_vec {
            vols_str.push_str(vol.to_json().as_str());
            vols_str.push_str(",");
        }
        vols_str = vols_str[0..(vols_str.len()-1)].to_string();
        vols_str.push_str("]");
        vols_str
    };
    
    let reponse = format!(
        "{}\r\nContent-Length: {}\n\
        Content-Type: application/json\
        \nAccess-Control-Allow-Origin: *\r\n\r\n{}",
        ligne_statut,
        contenu.len(),
        contenu
    );

    
    flux.write(reponse.as_bytes()).unwrap();
    flux.flush().unwrap();

    let requetes_en_cours_lock = requetes_en_cours.lock().unwrap();
    enlever_requete(requetes_en_cours_lock.to_vec(), adresse);
    drop(requetes_en_cours_lock);
}

mod tests;