use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::fs;
use std::thread;
mod ogn;
use ogn::thread_ogn;
use serveur::{ajouter_requete, enlever_requete, Vol};
use std::sync::{Arc, Mutex};



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
    let get = b"GET / HTTP/1.1\r\n";

    let (ligne_statut, nom_fichier) = if tampon.starts_with(get) {
        ("HTTP/1.1 200 OK", "example.html")
    } else {
        ("HTTP/1.1 404 NOT FOUND", "404.html")
    };
    

    let contenu= fs::read_to_string(nom_fichier).unwrap();

    let reponse = format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
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