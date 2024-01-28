//! Tout ce qui attrait aux flights que nous enregistrons.

use brick_ogn::flight::Flight;
use chrono::NaiveTime;



/// A trait to update a list of Flights with the same list but with newer infomations. 
/// can be useful when we got a new list of fligths from OGN but the takeoff
/// and landing times chaged.
pub trait Update {
    /// Update a vector of flights.
    fn update(&mut self, nouveaux_flights: Vec<Flight>);
}

impl Update for Vec<Flight> {
    fn update(&mut self, derniers_flights: Vec<Flight>) {
        // Testing equality and replacing if needed.
        let mut index_next_flight = 0;
        let mut priority_next_flight = 0;
        #[allow(unused_assignments)]
        for (mut index_new_flight, new_flight) in derniers_flights.into_iter().enumerate() {
            let mut exists = false;
            for old_flight in &mut *self {
                // if on the same flight
                if new_flight.ogn_nb == old_flight.ogn_nb {
                    exists = true;
                    let default_time = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
                    //teste les différentes valeurs qui peuvent être mises a day
                    if old_flight.takeoff == default_time {
                        old_flight.takeoff = new_flight.takeoff;
                    }
                    if old_flight.landing == default_time {
                        old_flight.landing = new_flight.landing;
                    }
                } else if new_flight.glider == old_flight.glider {
                    if priority_next_flight != 0 {
                        if priority_next_flight < new_flight.ogn_nb
                            && new_flight.ogn_nb < 0
                        {
                            exists = true;
                            priority_next_flight = new_flight.ogn_nb;
                            index_next_flight = index_new_flight;
                        }
                    } else if new_flight.ogn_nb < 0 && priority_next_flight == 0 {
                        exists = true;
                        priority_next_flight = new_flight.ogn_nb;
                        index_next_flight = index_new_flight;
                    }
                }
            }
            if priority_next_flight != 0 {
                // we get the highest priority flight and put in the New OGN data
                self[index_next_flight].ogn_nb = new_flight.ogn_nb;
                self[index_next_flight].takeoff_code = new_flight.takeoff_code.clone();
                self[index_next_flight].takeoff = new_flight.takeoff;
                self[index_next_flight].landing = new_flight.landing;
            }
            if !exists {
                self.push(new_flight);
            }
            index_new_flight += 1;
        }
    }
}
