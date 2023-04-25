use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::fs;
use std::thread;
use chrono::prelude::*;
mod ogn;
use ogn::{requete_ogn, traitement_requete_ogn};
use serveur::{ajouter_requete, enlever_requete};
use std::sync::{Arc, Mutex};


fn main() {

    let mut requetes_en_cours: Arc<Mutex<Vec<serveur::Client>>> = Arc::new(Mutex::new(Vec::new()));
    let ecouteur = TcpListener::bind("127.0.0.1:7878").unwrap();



    //ca dans un thread
    let date = NaiveDate::from_ymd_opt(2023, 04, 25).unwrap();
    traitement_requete_ogn(date, requete_ogn(date));
        

    for flux in ecouteur.incoming() {
        let flux = flux.unwrap();

        let adresse = format!("{}", (flux.peer_addr().unwrap()));
        let requetes_en_cours = requetes_en_cours.clone();
        let adresse = adresse.clone();
        thread::spawn(move || {
            let requetes_en_cours_lock = requetes_en_cours.lock().unwrap();
            ajouter_requete(requetes_en_cours_lock.to_vec(), adresse.clone());
            drop(requetes_en_cours_lock);
            
            gestion_connexion(flux);

            let requetes_en_cours_lock = requetes_en_cours.lock().unwrap();
            enlever_requete(requetes_en_cours_lock.to_vec(), adresse);
            drop(requetes_en_cours_lock);
        });
    }
}
        

fn gestion_connexion(mut flux: TcpStream) {

    let mut tampon = [0; 1024];

    flux.read(&mut tampon).unwrap();
    let get = b"GET / HTTP/1.1\r\n";

    let (ligne_statut, nom_fichier) = if tampon.starts_with(get) {
        ("HTTP/1.1 200 OK", "example.html")
    } else {
        ("HTTP/1.1 404 NOT FOUND", "404.html")
    };
    //println!("Requête : {}", String::from_utf8_lossy(&tampon[..]));

    let contenu= fs::read_to_string(nom_fichier).unwrap();

    let reponse = format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        ligne_statut,
        contenu.len(),
        contenu
    );
    flux.write(reponse.as_bytes()).unwrap();
    flux.flush().unwrap();   
}

mod tests;