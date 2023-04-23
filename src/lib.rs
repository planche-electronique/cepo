use std::fs;
use json::JsonValue::Array;
use chrono::prelude::*;
use std::sync::mpsc::{Receiver, Sender};

pub struct Vol {
    pub numero_ogn: i32,
    pub aeronef: String,
    pub decollage: NaiveTime,
    pub atterissage: NaiveTime,
}

impl Vol {
    fn default() -> Self {
        Vol {
            numero_ogn: 0,
            aeronef: "X-XXXX".to_string(),
            decollage: NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            atterissage: NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        }
    }

    pub fn to_json(self: &Self) -> String {
        let vol = json::object!{
            numero_ogn: self.numero_ogn,
            aeronef: *self.aeronef,
            decollage: *self.decollage.format("%Hh%M").to_string(),
            atterissage: *self.atterissage.format("%Hh%M").to_string(),
        };
        vol.dump()
    }
}

pub struct Appareil {
    pub modele: String,
    pub categorie: u8,
    pub immatriculation: String,
}


fn liste_immatriculations() -> Vec<String> {
    let contenu_fichier = fs::read_to_string("immatriculations.json")
        .expect("Probleme lors de la leture du fichier");
    let fichier_parse = json::parse(contenu_fichier.as_str()).unwrap();
    let immatriculations_json = match fichier_parse {   
        Array(vecteur) => {
            vecteur
        },
        _ => {
            eprintln!("immatriculations.json n'est pas un tableau");
            Vec::new()
        },
    };
    let mut immatriculations = Vec::new();
    for immatriculation_json in immatriculations_json {
        match immatriculation_json {
            json::JsonValue::Short(immatriculation) => {
                immatriculations.push(immatriculation.as_str().to_string());
            },
            _ => {
                eprintln!("{} n'est pas de type short", immatriculation_json);
            }
        }
    }
    immatriculations
}

fn thread_gestion(tx_co: Sender<String>, rx_co: Receiver<String>) {
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
}

#[derive(Clone, PartialEq)]
struct Client {
    adresse: String,
    requetes_en_cours: i32,
}
