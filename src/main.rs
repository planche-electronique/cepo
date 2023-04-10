use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::fs;

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

    let contenu = fs::read_to_string("example.html").unwrap();

    let reponse = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
        contenu.len(),
        contenu
    );

    flux.write(reponse.as_bytes()).unwrap();
    flux.flush().unwrap();
}
