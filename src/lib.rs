#![warn(missing_docs)]

//! Easy and fast usage of OGN data to load and save takeoffs and landing of glider flights.
//! The program reads under `$XDG_DAT_DIR/cepo/infos.json` to get a list of pilots, 
//! names, immatriculations to look at, takeoff_machines and pilots etc.

use crate::client::Client;
use std::fs;

use std::sync::{Arc, Mutex};

use brick_ogn::flightlog::update::Update;
use brick_ogn::flightlog::FlightLog;

pub mod client;
pub mod ogn;
pub mod flightlog;
pub mod flight;

/// Aircraft struct, used to parse OGN API.
pub struct Aircraft {
    /// The type of the aircraft, coming from OGN.
    pub modele: String,
    /// Aircraft category (airplane, glider...) using OGN codes from 
    /// [there](https://gitlab.com/davischappins/ogn-flightbook/-/blob/master/doc/API.md.)
    pub categorie: u8,
    /// The string of the immatriculation e(ex: `F-CMOI`).
    pub immatriculation: String,
}

/// Ajoute un 0 devant le nombre s'il est inférieur à 10 pour avoir des strings à 2 chiffres et à longueur fixe.
/// # Exemple
/// ```
/// use serveur::{nb_2digits_string};
/// assert_eq!(nb_2digits_string(2), String::from("02"));
/// assert_eq!(nb_2digits_string(20), String::from("20"));
/// ```
pub fn nb_2digits_string(nombre: i32) -> String {
    if nombre > 9 {
        nombre.to_string()
    } else {
        format!("0{}", nombre)
    }
}

/// Create the path associated with a day of the time at "$XDG_DATA_DIR/cepo/year/month/day".
pub fn create_fs_path_day(annee: i32, mois: u32, jour: u32) {
    let jour_str = nb_2digits_string(jour as i32);
    let mois_str = nb_2digits_string(mois as i32);

    let mut path = crate::data_dir();
    path.push(annee.to_string());
    path.push(&mois_str);
    path.push(&jour_str);

    if !path.as_path().exists() {
        dbg!(&path);
        fs::create_dir_all(&path).unwrap();
        log::info!("Création du chemin {}/{}/{}", annee, &mois_str, &jour_str);
    }
}

/// Allows to store and share configuration of the server. Loaded thanks to
/// [confy](https://crates.io/crates/confy). Default value is written if there 
/// is no config file.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Configuration {
    /// OACI string of the airport to look at.
    pub oaci: String,
    /// Time between each OGN poll.
    pub f_synchronisation_secs: i32,
    /// The port on which the server will listen to requests (default to 7878).
    pub port: i32,
    /// Le log level to show. Default is "info".  To choose between trace",
    /// "debug", "info", "warn", "error".
    pub niveau_log: String,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            oaci: "LFLE".to_string(),
            f_synchronisation_secs: 300,
            port: 7878,
            niveau_log: "info".to_string(),
        }
    }
}

/// Server context. Stores configuration, current requests, updates made in the 
/// after the last OGN request and the FlightLog of the day.
#[derive(Clone)]
pub struct Context {
    /// Server config.
    pub configuration: Configuration,
    /// The day flightlog.
    pub flightlog: Arc<Mutex<FlightLog>>,
    /// An vector of Update to keep in memory the updates that were recently made
    /// (after the last OGN automatic request) to avoid to reload the entire 
    /// flightlog.
    pub updates: Arc<Mutex<Vec<Update>>>,
    /// A vector that stores who is actually requesting, to limit the number of 
    /// concurrent request of the same user. (Some sort of ddos protection).
    pub current_requests: Arc<Mutex<Vec<Client>>>,
}

/// A function that provides the basic path for storage using dirs crate to
/// provide platform specific paths.
pub fn data_dir() -> std::path::PathBuf {
    let mut data_dir = dirs::data_dir().expect(
        "Couldn't guess where to store files. Check your os compatibility \
            with dirs (https://crates.io/crates/dirs) crate.",
    );
    data_dir.push("cepo");
    data_dir
}
