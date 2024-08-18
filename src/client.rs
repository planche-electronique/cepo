//! Storage and associated methods of a Client: a machine that is using the service.

use log;
use std::{
    net::IpAddr,
    sync::{Arc, Mutex},
};

/// Trait that allows to change the number of active requests a [`Client`] is havingClient`].
pub trait UsageControl {
    /// Increase by one the counter of active request of a [`Client`] and create
    /// if needed the entry for him in [`self`].
    fn increase_usage(&mut self, adresse: &IpAddr) -> bool;
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
    fn increase_usage(&mut self, adresse: &IpAddr) -> bool {
        let mut i = 0;
        while i < self.len() && self[i].adresse != *adresse {
            i += 1;
        }
        if i == self.len() {
            self.push(Client {
                adresse: *adresse,
                requetes_en_cours: 1,
            });
            log::info!("Incoming connection from {}.", adresse);
            true
        } else if self[i].adresse == *adresse {
            if self[i].requetes_en_cours < 10 {
                self[i].requetes_en_cours += 1;
                log::info!("Handling request {}; add to register.", &adresse);
                true
            } else {
                log::warn!("No more request authorized for {}", adresse);
                false
            }
        } else {
            true
        }
    }

    fn decrease_usage(&mut self, adresse: &IpAddr) {
        let mut i = 0;
        while i < self.len() && self[i].adresse != *adresse {
            i += 1;
        }
        if i == self.len() {
        } else if self[i].adresse == *adresse {
            if self[i].requetes_en_cours != 1 {
                self[i].requetes_en_cours -= 1;
            } else {
                self.remove(i);
            }
            log::info!("End of request for {}", adresse);
        }
    }
}

impl UsageControl for Arc<Mutex<Vec<Client>>> {
    fn increase_usage(&mut self, adresse: &IpAddr) -> bool {
        let mut requetes_en_cours_lock = self.lock().unwrap();
        let res = (*requetes_en_cours_lock).increase_usage(adresse);
        drop(requetes_en_cours_lock);
        res
    }

    fn decrease_usage(&mut self, adresse: &IpAddr) {
        let mut requetes_en_cours_lock = self.lock().unwrap();
        (*requetes_en_cours_lock).decrease_usage(adresse);
        drop(requetes_en_cours_lock);
    }
}
