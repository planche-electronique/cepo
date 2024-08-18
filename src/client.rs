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
    fn increase_usage(&mut self, address: &IpAddr) -> bool;
    /// Decrease by one the counter of active request of a [`Client`] and delete
    /// if needed the entry for him in [`self`].
    fn decrease_usage(&mut self, address: &IpAddr);
}

/// Structure of a client.
#[derive(Clone, PartialEq)]
pub struct Client {
    /// Ip address as returned by hyper.
    address: IpAddr,
    /// The number of requests the client is actually making and are processing.
    current_requests: i32,
}

impl UsageControl for Vec<Client> {
    fn increase_usage(&mut self, address: &IpAddr) -> bool {
        let mut i = 0;
        while i < self.len() && self[i].address != *address {
            i += 1;
        }
        if i == self.len() {
            self.push(Client {
                address: *address,
                current_requests: 1,
            });
            log::info!("Incoming connection from {}.", address);
            true
        } else if self[i].address == *address {
            if self[i].current_requests < 10 {
                self[i].current_requests += 1;
                log::info!("Handling request {}; add to register.", &address);
                true
            } else {
                log::warn!("No more request authorized for {}", address);
                false
            }
        } else {
            true
        }
    }

    fn decrease_usage(&mut self, address: &IpAddr) {
        let mut i = 0;
        while i < self.len() && self[i].address != *address {
            i += 1;
        }
        if i == self.len() {
        } else if self[i].address == *address {
            if self[i].current_requests != 1 {
                self[i].current_requests -= 1;
            } else {
                self.remove(i);
            }
            log::info!("End of request for {}", address);
        }
    }
}

impl UsageControl for Arc<Mutex<Vec<Client>>> {
    fn increase_usage(&mut self, address: &IpAddr) -> bool {
        let mut current_requests_lock = self.lock().unwrap();
        let res = (*current_requests_lock).increase_usage(address);
        drop(current_requests_lock);
        res
    }

    fn decrease_usage(&mut self, address: &IpAddr) {
        let mut current_requests_lock = self.lock().unwrap();
        (*current_requests_lock).decrease_usage(address);
        drop(current_requests_lock);
    }
}
