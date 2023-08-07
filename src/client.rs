//! Stockage et méthodes associées à un client: une machine qui fait des requêtes en cours.

use log;
use std::sync::{Arc, Mutex};

/// Trait qui permet de changer le nombre de requêtes associées à un [`Client`].
pub trait VariationRequete {
    /// Incrémente de 1 le compteur de requêtes en cours d'un [`Client`] et crée si besoin 
    /// l'entrée pour celui-ci dans [`self`].
    fn incrementer(&mut self, adresse: String);
    /// Décrémente de 1 le compteur de requêtes en cours d'un [`Client`] et supprime si besoin
    /// l'entrée de celui-ci dans [`self`].
    fn decrementer(&mut self, adresse: String);
}

/// Structure associée à un client.
#[derive(Clone, PartialEq)]
pub struct Client {
    /// L'adresse ip du client.
    adresse: String,
    /// Le nombre de requêtes que le client a faite et qui sont en cours de traitement.
    requetes_en_cours: i32,
}

impl VariationRequete for Vec<Client> {
    fn incrementer(&mut self, adresse: String) {
        //println!("+1 connection : {}", adresse.clone());
        let mut adresse_existe: bool = false;
        for mut client in self.clone() {
            if client.adresse == adresse {
                if client.requetes_en_cours < 10 {
                    client.requetes_en_cours += 1;
                    adresse_existe = true;
                    log::info!(
                        "Traitement de la requete de {}; ajout au registre.",
                        client.adresse.clone()
                    );
                } else {
                    log::info!("pas plus de requêtes pour {}", adresse);
                }
            }
            if !adresse_existe {
                self.push(Client {
                    adresse: adresse.to_string(),
                    requetes_en_cours: 1,
                });
                log::info!("Ajout de l'adresse : {} au registre", adresse.clone());
            }
        }
    }

    fn decrementer(&mut self, adresse: String) {
        for mut client in self.clone() {
            if client.adresse == adresse {
                if client.requetes_en_cours != 1 {
                    client.requetes_en_cours -= 1;
                } else {
                    let index = self.iter().position(|x| *x == client).unwrap();
                    self.remove(index);
                }
                log::info!("Fin de requete pour {}", adresse.clone());
            }
        }
    }
}

impl VariationRequete for Arc<Mutex<Vec<Client>>> {
    fn incrementer(&mut self, adresse: String) {
        let requetes_en_cours_lock = self.lock().unwrap();
        requetes_en_cours_lock.to_vec().incrementer(adresse.clone());
        drop(requetes_en_cours_lock);
    }

    fn decrementer(&mut self, adresse: String) {
        let requetes_en_cours_lock = self.lock().unwrap();
        requetes_en_cours_lock.to_vec().decrementer(adresse.clone());
        drop(requetes_en_cours_lock);
    }
}
