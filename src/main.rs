use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::fs;
use std::thread;
use std::sync::{mpsc};
use chrono::prelude::*;
mod ogn;
use ogn::{requete_ogn, traitement_requete_ogn};
use serveur::thread_gestion;


fn main() {
    let ecouteur = TcpListener::bind("127.0.0.1:7878").unwrap();

    let (tx_main, rx_co) = mpsc::channel();
    let (tx_co, rx_main) = mpsc::channel();


    //ca dans un thread
    let date = NaiveDate::from_ymd_opt(2023, 04, 20).unwrap();
    traitement_requete_ogn(date, requete_ogn(date));

    thread::spawn(move || {
        thread_gestion(tx_co, rx_co);
    });
        

    for flux in ecouteur.incoming() {
        let flux = flux.unwrap();

        let adresse = format!("{}", (flux.peer_addr().unwrap()));
        let tx_main = tx_main.clone();
        let adresse = adresse.clone();
        tx_main.send(format!("+{}", adresse).to_owned()).unwrap();
        match rx_main.recv().unwrap() {
            string => {
                if string.as_str() == "Ok"{
                    thread::spawn(move || {
                        gestion_connexion(flux);
                        tx_main.send(format!("-{}", adresse).to_owned()).unwrap_or_else(|err| {
                            eprintln!("erreur à l'envoi du message de {} : {}"
                                , adresse, err);
                        });
                    });
                };
            }
        }
        
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