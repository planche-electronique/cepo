use std::io::prelude::*;
use std::net::{TcpListener, TcpStream, SocketAddr};
use std::fs;
use std::thread;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;


fn main() {
    let ecouteur = TcpListener::bind("127.0.0.1:7878").unwrap();

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let mut requetes_en_cours: Vec<Client> = Vec::new();
        while true {
            let message: String = rx.recv().unwrap();
            let signe = &message[0..1];
            let adresse = &message[1..message.len()];
            match signe {
                "+" => {
                    println!("une connection de gagnee pour {}", adresse);
                    let mut est_active: bool = false;
                    for mut client in requetes_en_cours.clone() {
                        if client.adresse == adresse {
                            client.requetes_en_cours += 1;
                            est_active = true;
                        }
                        if est_active == false {
                            requetes_en_cours.push(Client {
                                adresse: adresse.to_string(),
                                requetes_en_cours: 1,
                            })
                        }
                    }
                },
                "-" => {
                    println!("une connection de perdue pour {}", adresse);
                    for mut client in requetes_en_cours.clone() {
                        if client.adresse == adresse {
                            if client.requetes_en_cours != 1{
                                client.requetes_en_cours -=1;
                            } else {
                                let index = requetes_en_cours.iter().position(|x| *x == client).unwrap();
                                requetes_en_cours.remove(index);
                            }
                            
                        }
                    }
                },
                _ => eprintln!("not a valid message"),
            }
        }
    });
        

    for flux in ecouteur.incoming() {
        let flux = flux.unwrap();

        let adresse = format!("{}", (flux.peer_addr().unwrap()));
        let tx = tx.clone();

        thread::spawn(move || {
            let adresse = adresse.clone();
            tx.send(format!("+{}", adresse).to_owned()).unwrap();
            gestion_connexion(flux);
            tx.send(format!("-{}", adresse).to_owned()).unwrap_or_else(|err| {
                eprintln!("erreur à l'envoi du message de ")
            });
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

fn retourner_message_erreur(mut flux: TcpStream) {
    let reponse = format!(
        "HTTP/1.1 408 REQUEST TIME-OUT\r\n"
    );
    flux.write(reponse.as_bytes()).unwrap();
    flux.flush().unwrap();
}

#[derive(Clone, PartialEq)]
struct Client {
    adresse: String,
    requetes_en_cours: i32,
}
