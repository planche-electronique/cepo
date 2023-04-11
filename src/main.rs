use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::fs;
use std::thread;
use std::sync::{mpsc};


fn main() {
    let ecouteur = TcpListener::bind("127.0.0.1:7878").unwrap();

    let (tx_main, rx_co) = mpsc::channel();
    let (tx_co, rx_main) = mpsc::channel();

    thread::spawn(move || {
        let mut requetes_en_cours: Vec<Client> = Vec::new();
        loop {
            let message: String = rx_co.recv().unwrap();
            let signe = &message[0..1];
            let adresse = &message[1..message.len()];
            match signe {
                "+" => {
                    println!("une connection de gagnee pour {}", adresse);
                    let mut est_active: bool = false;
                    for mut client in requetes_en_cours.clone() {
                        if client.adresse == adresse {
                            if client.requetes_en_cours < 10 {
                                client.requetes_en_cours += 1;
                                est_active = true;
                                tx_co.send("Ok".to_string()).unwrap();
                            } else {
                                println!("pas plus de requêtes pour {}", adresse);
                                tx_co.send("No".to_string()).unwrap();
                            }
                        }
                        if est_active == false {
                            requetes_en_cours.push(Client {
                                adresse: adresse.to_string(),
                                requetes_en_cours: 1,
                            });
                            tx_co.send("Ok".to_string()).unwrap();
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
                    tx_co.send("Ok".to_string()).unwrap();
                },
                _ => eprintln!("not a valid message"),
            }
        }
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

#[derive(Clone, PartialEq)]
struct Client {
    adresse: String,
    requetes_en_cours: i32,
}
