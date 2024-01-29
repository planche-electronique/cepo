//! Storage and associated methods of a Client: a machine that is using the service.

use log;
use std::{net::IpAddr, sync::{Arc, Mutex}};

/// Trait that allows to change the number of active requests a [`Client`] is havingClient`].
pub trait UsageControl {
    /// Increase by one the counter of active request of a [`Client`] and create
    /// if needed the entry for him in [`self`].
    fn increase_usage(&mut self, adresse: &IpAddr);
    /// Decrease by one the counter of active request of a [`Client`] and delete
    /// if needed the entry for him in [`self`].
    fn decrease_usage(&mut self, adresse: &IpAddr);
}

/// Structure of a client.
#[derive(Clone, PartialEq)]
pub struct Client {
    /// Ip address as returned by hyper.
    adresse: IpAddr,
    /// The number of requests the client is actually making and are processing.
    requetes_en_cours: i32,
}

impl UsageControl for Vec<Client> {
    fn increase_usage(&mut self, adresse: &IpAddr) {
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

    fn decrease_usage(&mut self, adresse: &IpAddr) {
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

impl UsageControl for Arc<Mutex<Vec<Client>>> {
    fn increase_usage(&mut self, adresse: &IpAddr) {
        let mut requetes_en_cours_lock = self.lock().unwrap();
        (*requetes_en_cours_lock).increase_usage(adresse);
        drop(requetes_en_cours_lock);
    }

    fn decrease_usage(&mut self, adresse: &IpAddr) {
        let mut requetes_en_cours_lock = self.lock().unwrap();
        (*requetes_en_cours_lock).decrease_usage(adresse);
        drop(requetes_en_cours_lock);
    }
}
