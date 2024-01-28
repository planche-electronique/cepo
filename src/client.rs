//! Stockage et méthodes associées à un client: une machine qui fait des requêtes en cours.

use log;
use std::{net::IpAddr, sync::{Arc, Mutex}};

/// Trait qui permet de changer le nombre de requêtes associées à un [`Client`].
pub trait VariationRequete {
    /// Incrémente de 1 le compteur de requêtes en cours d'un [`Client`] et crée si besoin 
    /// l'entrée pour celui-ci dans [`self`].
    fn incrementer(&mut self, adresse: &IpAddr);
    /// Décrémente de 1 le compteur de requêtes en cours d'un [`Client`] et supprime si besoin
    /// l'entrée de celui-ci dans [`self`].
    fn decrementer(&mut self, adresse: &IpAddr);
}

/// Structure associée à un client.
#[derive(Clone, PartialEq)]
pub struct Client {
    /// L'adresse ip du client.
    adresse: IpAddr,
    /// Le nombre de requêtes que le client a faite et qui sont en cours de traitement.
    requetes_en_cours: i32,
}

impl VariationRequete for Vec<Client> {
    fn incrementer(&mut self, adresse: &IpAddr) {
        //println!("+1 connection : {}", adresse.clone());
        let mut adresse_existe: bool = false;
        for mut client in self.clone() {
            if client.adresse == *adresse {
                if client.requetes_en_cours < 10 {
                    client.requetes_en_cours += 1;
                    adresse_existe = true;
                    log::info!(
                        "Handling request {}; add to register.",
                        &adresse
                    );
                } else {
                    log::info!("No more request authorized for {}", adresse);
                }
            }
        }
        if !adresse_existe {
            self.push(Client {
                adresse: *adresse,
                requetes_en_cours: 1,
            });
            log::info!("Adding address : {} to the connection log.", adresse);
        }
    }

    fn decrementer(&mut self, adresse: &IpAddr) {
        for mut client in self.clone() {
            if client.adresse == *adresse {
                if client.requetes_en_cours != 1 {
                    client.requetes_en_cours -= 1;
                } else {
                    let index = self.iter().position(|x| *x == client).unwrap();
                    self.remove(index);
                }
                log::info!("End of request for {}", adresse);
            }
        }
    }
}

impl VariationRequete for Arc<Mutex<Vec<Client>>> {
    fn incrementer(&mut self, adresse: &IpAddr) {
        let mut requetes_en_cours_lock = self.lock().unwrap();
        (*requetes_en_cours_lock).incrementer(adresse);
        drop(requetes_en_cours_lock);
    }

    fn decrementer(&mut self, adresse: &IpAddr) {
        let mut requetes_en_cours_lock = self.lock().unwrap();
        (*requetes_en_cours_lock).decrementer(adresse);
        drop(requetes_en_cours_lock);
    }
}
