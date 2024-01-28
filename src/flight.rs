//! Tout ce qui attrait aux flights que nous enregistrons.

use crate::{create_fs_path_day, data_dir, nb_2digits_string, Context};
use async_trait::async_trait;
use brick_ogn::flight::Flight;
use chrono::{Datelike, NaiveDate, NaiveTime};
use std::fs;
use serde_json;


/// How to save the flight log on a computer.
#[async_trait]
pub trait FlightSaving {
    /// Saving flights from a date to the path `$XDG_DATA_DIR/cepo/year/month/day`.
    fn save(&self, date: NaiveDate);
    /// Loading flight from filesystem at `$XDG_DATA_DIR/cepo/year/month/day`.
    fn load(date: NaiveDate)
        -> Result<Vec<Flight>, Box<dyn std::error::Error + Send + Sync>>;
    /// Loads flights from filesystem and alsoupdates them through OGN.
    async fn from_day(
        date: NaiveDate,
        context: &Context,
    ) -> Result<Vec<Flight>, Box<dyn std::error::Error + Send + Sync>>;
}

#[async_trait]
impl FlightSaving for Vec<Flight> {
    fn save(&self, date: NaiveDate) {
        let flights = self.clone();
        let year = date.year();
        let month = date.month();
        let day = date.day();

        let day_str = nb_2digits_string(day as i32);
        let month_str = nb_2digits_string(month as i32);

        log::info!(
            "Saving flights made on {}/{}/{}",
            year,
            month_str,
            day_str
        );

        create_fs_path_day(year, month, day);

        for (index, vol) in flights.iter().enumerate() {
            let index_str = nb_2digits_string(index as i32);
            let flight_string = serde_json::to_string(vol).unwrap_or_default();
            let mut flights_path = crate::data_dir();
            flights_path.push(format!("{year}/{month_str}/{day_str}/{index_str}.json"));
            let mut file = String::new();
            if flights_path.exists() {
                file = fs::read_to_string(&flights_path).unwrap_or_else(|err| {
                    log::error!(
                        "File number {} of path {:?} not found or could'nt be opened : {}",
                        index,
                        &flights_path,
                        err.to_string()
                    );
                    "".to_string()
                });
            }
            
            if file != flight_string {
                fs::write(&flights_path, flight_string).unwrap_or_else(|err| {
                    log::error!(
                        "Can't write file of day {}/{}/{} and index {} : {}",
                        year,
                        month_str,
                        day_str,
                        index,
                        err
                    );
                });
            }
        }
    }

    fn load(
        date: NaiveDate,
    ) -> Result<Vec<Flight>, Box<dyn std::error::Error + Send + Sync>> {
        let year = date.year();
        let month = date.month();
        let day = date.day();

        let month_str = nb_2digits_string(month as i32);
        let day_str = nb_2digits_string(day as i32);

        log::info!("Readng flight files of {year}/{month_str}/{day_str}");

        create_fs_path_day(year, month, day);
        let mut path = data_dir();
        path.push(format!("{}/{}/{}/", year, month_str, day_str));
        let files = fs::read_dir(&path).unwrap_or_else(|_| panic!("Couldn't load {:?}", path.clone()));
        let mut flights: Vec<Flight> = Vec::new();

        for file in files {
            let file_name = file.unwrap().file_name().into_string().unwrap();
            let file_path = path.as_path().join(std::path::Path::new(&file_name));
            if &file_name != "affectations.json" {
                let flight_str_json = fs::read_to_string(file_path).unwrap_or_else(|err| {
                    log::error!("Can't open file {} : {}", file_name, err);
                    String::from("")
                });
                let vol = serde_json::from_str(&flight_str_json)?;
                flights.push(vol);
            }
        }
        Ok(flights)
    }

    async fn from_day(
        date: NaiveDate,
        context: &Context,
    ) -> Result<Vec<Flight>, Box<dyn std::error::Error + Send + Sync>> {
        let flights = Vec::load(date).unwrap();
        // looks to be unuseful no ?
        //there should be a force trigger but it is normally complete as it is not today's flights
        //flights.mettre_a_day(flights_ogn(date, actif_serveur.configuration.oaci.clone()).await?);
        flights.save(date);
        Ok(flights)
    }
}

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
