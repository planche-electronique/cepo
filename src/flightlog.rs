//! FlightLog: an object to represent a group of flights and the organization
//! on the ground at the moment.

use crate::ogn::ogn_flights;
use crate::{create_fs_path_day, nb_2digits_string};
use async_trait::async_trait;
pub use brick_ogn::flightlog::update::Update;
use brick_ogn::flightlog::FlightLog;
use chrono::{Datelike, NaiveDate, NaiveTime};
use log;
use tokio::fs;
/// A trait that cares about the storage of a FlightLog on a computer.
#[async_trait]
pub trait Storage {
    /// FlightLog from the disk and updated from ogn.
    async fn from_day(
        date: NaiveDate,
        oaci: &String,
    ) -> Result<FlightLog, Box<dyn std::error::Error + Send + Sync>>;
    /// Updating the flightlog from ogn using today's date.
    async fn update_ogn(
        &mut self,
        oaci: &String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    /// Loading FlightLog from the disk only, without updating.
    async fn load(
        date: NaiveDate,
        oaci: &String,
    ) -> Result<FlightLog, Box<dyn std::error::Error + Send + Sync>>;
    /// Savinfg on the disk only, wuthout updating/
    async fn save(&self, oaci: &String);
}

#[async_trait]
impl Storage for FlightLog {
    /// Loads the flightlog from local files and updates it from the internet if needed
    async fn from_day(
        date: NaiveDate,
        oaci: &String,
    ) -> Result<FlightLog, Box<dyn std::error::Error + Send + Sync>> {
        let year: i32 = date.year();
        let month = date.month();
        let day = date.day();

        create_fs_path_day(year, month, day);
        let mut flightlog = FlightLog::load(date, oaci).await.unwrap_or_else(|err| {
            log::warn!("Could not load flightlog from disk, trying to update from OGN : {err}");
            let mut fl = FlightLog::default();
            fl.date = date;
            fl
        });
        match flightlog.update_ogn(oaci).await {
            Ok(_) => {
                let _ = flightlog.save(oaci);
            }
            Err(err) => {
                log::error!("Could not connect to OGN ! : {err}");
            }
        }
        let _ = flightlog.save(oaci);
        Ok(flightlog)
    }

    /// Updating the flightlog from ogn using today's date.
    async fn update_ogn(
        &mut self,
        oaci: &String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // We test equalities and we replace if needed.
        let last_flights_fut = ogn_flights(self.date, oaci.to_string());
        let mut index_next_flight = 0;
        let mut priority_next_flight = 0;
        let old_flightlog = self;
        #[allow(unused_assignments)]
        for (mut index_new_flight, new_flight) in last_flights_fut.await?.into_iter().enumerate() {
            let mut exists = false;
            for old_flight in &mut old_flightlog.flights {
                // if on the same flight
                if new_flight.ogn_nb == old_flight.ogn_nb {
                    exists = true;
                    let heure_default = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
                    //test the different values that can be updated
                    if old_flight.takeoff == heure_default {
                        old_flight.takeoff = new_flight.takeoff;
                    }
                    if old_flight.landing == heure_default {
                        old_flight.landing = new_flight.landing;
                    }
                } else if new_flight.glider == old_flight.glider {
                    if priority_next_flight != 0 {
                        if priority_next_flight < new_flight.ogn_nb && new_flight.ogn_nb < 0 {
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
                // We get the flight with the highest priority and we write on it the data from OGN.
                old_flightlog.flights[index_next_flight].ogn_nb = new_flight.ogn_nb;
                old_flightlog.flights[index_next_flight].takeoff_code =
                    new_flight.takeoff_code.clone();
                old_flightlog.flights[index_next_flight].takeoff = new_flight.takeoff;
                old_flightlog.flights[index_next_flight].landing = new_flight.landing;
            }
            if !exists {
                old_flightlog.flights.push(new_flight);
            }
            index_new_flight += 1;
        }
        Ok(())
    }

    /// Returns the flightlog from day and airfield that matches `oaci` from local files
    async fn load(
        date: NaiveDate,
        oaci: &String,
    ) -> Result<FlightLog, Box<dyn std::error::Error + Send + Sync>> {
        let year = date.year();
        let month = date.month();
        let day = date.day();
        log::info!(
            "Loading FlightLog at {} from the disk {}-{}-{}",
            oaci,
            year,
            month,
            day
        );

        let month_str = nb_2digits_string(month as i32);
        let day_str = nb_2digits_string(day as i32);

        let mut path = crate::data_dir();
        path.push(format!("{}/{}/{}/{}.json", year, month_str, day_str, oaci));

        if path.exists() {
            let flightlog_str = fs::read_to_string(path).await.unwrap_or_default();
            let flightlog = serde_json::from_str(&flightlog_str)?;
            Ok(flightlog)
        } else {
            Err("No FlightLog found for the date.".into())
        }
    }

    async fn save(&self, oaci: &String) {
        let date = self.date;
        let year = date.year();
        let month = date.month();
        let day = date.day();

        let day_str = nb_2digits_string(day as i32);
        let month_str = nb_2digits_string(month as i32);

        let mut file_path = crate::data_dir();
        file_path.push(format!("{}/{}/{}/{}.json", year, month_str, day_str, oaci));

        fs::write(&file_path, serde_json::to_string(self).unwrap_or_default())
            .await
            .unwrap();

        log::info!("Saved FlightLog of the {year}/{month_str}/{day_str}.");
    }
}
