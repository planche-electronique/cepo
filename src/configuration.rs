//! Configuration for the server.
//! You can monitor different airfields. For each of them, you can configure
//! lists of pilots, tow pilots, winch pilots, winches and aerotows.
//! You can specify if the airport is monitored at all time (like you would for
//! a main airport) or some days (like you would for an airport you go in stage
//! someday a year).
//! You can specify these lists of pilots etc. globally.

use crate::flightlog::Storage;
use brick_ogn::flightlog::FlightLog;
use chrono::NaiveDate;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// An enum about when to monitor an airspace for flights
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq)]
pub enum DayMonitor {
    /// Monitor the airport every day
    Always,
    /// Monitor the airport only in specified days in the `Vec<NaiveDate>`
    Days(Vec<NaiveDate>),
}

impl Default for DayMonitor {
    fn default() -> Self {
        return DayMonitor::Always;
    }
}

/// A struct storing an airport
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq)]
pub struct AirportConfiguration {
    /// The OACI code of the airport
    oaci: String,
    /// A vector of pilots that are likely to be in the flightlog
    pilots: Vec<String>,
    /// A vector of winch pilots that are likely to be in the flightlog of this
    /// airfield
    winch_pilots: Vec<String>,
    /// A vector of tow pilots that are likely to be in the flightlog of this
    /// airfield
    tow_pilots: Vec<String>,
    /// A vector of winches that are likely to be in the flightlog of this airfield
    winches: Vec<String>,
    /// A vector of aerotows that are likely to be in the flightlog of this airfield
    aerotows: Vec<String>,
    /// The conditions about when to monitor this airport:
    day_monitor: DayMonitor,
    /// The immatriculations of the aircraft that we will log
    immatriculations: Vec<String>,
}

impl Default for AirportConfiguration {
    fn default() -> Self {
        Self {
            oaci: String::new(),
            pilots: Vec::new(),
            winches: Vec::new(),
            winch_pilots: Vec::new(),
            aerotows: Vec::new(),
            tow_pilots: Vec::new(),
            day_monitor: DayMonitor::default(),
            immatriculations: Vec::new(),
        }
    }
}

impl AirportConfiguration {
    /// Returns the oaci code of the airport
    pub fn oaci(&self) -> String {
        return self.oaci.clone();
    }

    /// Returns the daymonitor field, i.e. wether the airport is logged all the
    /// time or on specific dates.s
    pub fn day_monitor(&self) -> DayMonitor {
        return self.day_monitor.clone();
    }
}

/// Allows to store and share configuration of the server. Loaded thanks to
/// [confy](https://crates.io/crates/confy). Default value is written if there
/// is no config file.
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq)]
pub struct Configuration {
    /// configuration of all airports to look at
    pub airfileds_configs: Vec<AirportConfiguration>,
    /// Time between each OGN poll.
    pub f_synchronisation_secs: i32,
    /// The port on which the server will listen to requests (default to 7878).
    pub port: i32,
    /// Le log level to show. Default is "info".  To choose between trace",
    /// "debug", "info", "warn", "error".
    pub log_level: String,
    /// A vector of pilots that you want to be in any flightlog.
    pub permanent_pilots: Vec<String>,
    /// A vector of winch pilots that you want to be in any flightlog.
    pub permanent_winch_pilots: Vec<String>,
    /// A vector of tow pilots that you want to be in any flightlog.
    pub permanent_tow_pilots: Vec<String>,
    /// A vector of winches that you want to be in any flightlog.
    pub permanent_winches: Vec<String>,
    /// A vector of aerotows that you want to be in any flightlog.
    pub permanent_aerotows: Vec<String>,
    /// The immatriculations  we always log regardless of the airport
    pub immatriculations: Vec<String>,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            airfileds_configs: vec![AirportConfiguration::default()],
            f_synchronisation_secs: 300,
            port: 7878,
            log_level: "error".to_string(),
            permanent_pilots: Vec::new(),
            permanent_winches: Vec::new(),
            permanent_aerotows: Vec::new(),
            permanent_tow_pilots: Vec::new(),
            permanent_winch_pilots: Vec::new(),
            immatriculations: Vec::new(),
        }
    }
}

impl Configuration {
    /// Example configuration file
    pub fn example() -> Self {
        Self {
            airfileds_configs: vec![
                AirportConfiguration {
                    oaci: String::from("LFLE"),
                    pilots: vec![String::from("Walt Disney"), String::from("Roy Disney")],
                    winch_pilots: vec![String::from("Walt Disney"), String::from("Roy Disney")],
                    tow_pilots: vec![String::from("Walt Disney"), String::from("Roy Disney")],
                    winches: vec![String::from("yellow"), String::from("green")],
                    aerotows: vec![String::from("red"), String::from("blue")],
                    day_monitor: DayMonitor::Always,
                    immatriculations: vec![
                        String::from("F-CEJU"),
                        String::from("F-CECY"),
                        String::from("F-CBAR"),
                        String::from("F-CHFL"),
                    ],
                },
                AirportConfiguration {
                    oaci: String::from("LFLB"),
                    pilots: vec![String::from("Thomas Edison"), String::from("Pablo Picasso")],
                    winch_pilots: vec![
                        String::from("Thomas Edison"),
                        String::from("Pablo Picasso"),
                    ],
                    tow_pilots: vec![String::from("Thomas Edison"), String::from("Pablo Picasso")],
                    winches: vec![String::from("purple"), String::from("pink")],
                    aerotows: vec![String::from("white"), String::from("black")],
                    day_monitor: DayMonitor::Days(vec![
                        NaiveDate::from_ymd_opt(2024, 6, 10).unwrap()
                    ]),
                    immatriculations: vec![
                        String::from("F-CEJU"),
                        String::from("F-CDYA"),
                        String::from("F-CHBY"),
                        String::from("F-CLIN"),
                        String::from("F-CGCZ"),
                        String::from("F-CHFM"),
                    ],
                },
            ],
            f_synchronisation_secs: 300,
            port: 7878,
            log_level: "info".to_string(),
            permanent_pilots: vec![String::from("Steve Jobs"), String::from("Jony Ive")],
            permanent_winches: vec![String::from("brown"), String::from("orange")],
            permanent_tow_pilots: vec![String::from("Steve Jobs"), String::from("Jony Ive")],
            permanent_aerotows: vec![String::from("cyan"), String::from("clear green")],
            permanent_winch_pilots: vec![String::from("Steve Jobs"), String::from("Jony Ive")],
            immatriculations: vec![
                String::from("F-CVIP"),
                String::from("F-CNON"),
                String::from("F-CLMT"),
            ],
        }
    }

    /// Returns a HashMap containing flightlogs associated with their oaci code in String
    pub async fn create_needed_flightlog_hashmap(&self) -> HashMap<String, Arc<Mutex<FlightLog>>> {
        let mut hm = HashMap::new();
        for airport_config in &self.airfileds_configs {
            let date_today = chrono::Local::now().date_naive();
            let flightlog = FlightLog::load(date_today, &airport_config.oaci)
                .await
                .unwrap_or_else(|_| {
                    let mut fl = FlightLog::new();
                    fl.date = date_today;
                    fl
                });
            let flightlog_arc: Arc<Mutex<FlightLog>> = Arc::new(Mutex::new(flightlog));
            hm.insert(airport_config.oaci.clone(), flightlog_arc);
        }
        return hm;
    }
}

/// Copies an example configuration file instead of the actual config
pub fn copy_example_configuration_file() -> Result<(), confy::ConfyError> {
    let example = Configuration::example();
    confy::store("cepo", None, example)?;
    Ok(())
}
