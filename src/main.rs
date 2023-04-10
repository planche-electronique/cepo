use std::io::prelude::*;
use std::net::{TcpListener, TcpStream, SocketAddr};
use std::fs;
use std::thread;
use std::sync::{Arc, Mutex};
use db_interaction_server::GroupeTaches;

fn main() {
    let ecouteur = TcpListener::bind("127.0.0.1:7878").unwrap();
    let groupe = GroupeTaches::new(4);

    for flux in ecouteur.incoming() {
        groupe.executer(|| {
            gestion_connexion(&flux.unwrap());
        });
    }
}

fn gestion_connexion(flux: &TcpStream) {

    let mut tampon = [0; 1024];

    flux.read(&mut tampon).unwrap();
    let get = b"GET / HTTP/1.1\r\n";

    let (ligne_statut, nom_fichier) = if tampon.starts_with(get) {
        ("HTTP/1.1 200 OK", "example.html")
    } else {
        ("HTTP/1.1 404 NOT FOUND", "404.html")
    };
    println!("Requête : {}", String::from_utf8_lossy(&tampon[..]));

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

struct client {
    addresse: SocketAddr,
    requetes_en_cours: i32,
}