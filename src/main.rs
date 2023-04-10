use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};

fn main() {
    let ecouteur = TcpListener::bind("127.0.0.1:7878").unwrap();

    for flux in ecouteur.incoming() {
        let flux = flux.unwrap();

        gestion_conneexion(flux);
    }
}

fn gestion_conneexion(mut flux: TcpStream) {
    let mut tampon = [0; 1024];

    flux.read(&mut tampon).unwrap();

    println!("Requête : {}", String::from_utf8_lossy(&tampon[..]));
}
