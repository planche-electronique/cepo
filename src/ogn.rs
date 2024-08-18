//! To request ogn

use crate::flightlog::Storage;
use crate::Context;
use std::sync::{Arc, Mutex};

use crate::Aircraft;
use brick_ogn::flight::Flight;
use brick_ogn::flightlog::FlightLog;
use chrono::prelude::*;
use json::JsonValue;
use log;

/// Returns Flights that we requested to OGN and these are sorted
pub async fn ogn_flights(
    date: NaiveDate,
    immatriculations: Vec<String>,
    oaci: String,
) -> Result<Vec<Flight>, hyper::Error> {
    log::info!(
        "Requete à http://flightbook.glidernet.org/api/logbook/{}/{}",
        oaci,
        date.format("%Y-%m-%d").to_string()
    );
    let client = hyper::Client::new();
    let chemin = format!(
        "http://flightbook.glidernet.org/api/logbook/{}/{}",
        oaci,
        date.format("%Y-%m-%d")
    )
    .parse::<hyper::Uri>()
    .unwrap();
    let reponse = client.get(chemin).await?;
    let bytes = hyper::body::to_bytes(reponse.into_body()).await?;
    let corps_str = std::str::from_utf8(&bytes).unwrap().to_string();

    let requete_parse = json::parse(corps_str.as_str()).unwrap();
    log::info!("Traitement de la requete.");

    /* making link between number and immatriculations because ogn identifies
    aircrafts with numbers */
    let devices = requete_parse["devices"].clone();
    let mut appareils_ogn: Vec<Aircraft> = Vec::new();
    let tableau_devices = match devices {
        JsonValue::Array(appareils_json) => appareils_json,
        _ => {
            eprintln!("devices n'est pas un tableau");
            Vec::new()
        }
    };

    for appareil in tableau_devices {
        let modele_json = appareil["aircraft"].clone();
        let modele = modele_json.as_str().unwrap_or_default().to_string();

        let category_json = appareil["aircraft_type"].clone();
        let category = category_json.as_u8().unwrap();

        let immatriculation_json = appareil["registration"].clone();
        let immatriculation = immatriculation_json
            .as_str()
            .unwrap_or_default()
            .to_string();

        let appareil_actuel = Aircraft {
            modele,
            category,
            immatriculation,
        };
        appareils_ogn.push(appareil_actuel);
    }

    /* listing flights to give them immatriculations */

    let mut vols: Vec<Flight> = Vec::new();
    let flights = requete_parse["flights"].clone();
    let vols_json = match flights {
        JsonValue::Array(vols_json) => vols_json,
        _ => {
            eprintln!("La requete ogn n'a pas fourni un tableau.");
            Vec::new()
        }
    };

    for (mut index, vol_json) in vols_json.clone().into_iter().enumerate() {
        index += 1;

        // getting necessary fields
        let device = vol_json["device"].clone();
        let device_number = device.as_u8().unwrap() as usize;
        let immatriculation = appareils_ogn[device_number].immatriculation.clone();
        if !(immatriculations
            .iter()
            .any(|immat| *immat == immatriculation.clone()))
        {
            //Don't take immatriculation into account if not in list
            continue;
        }
        // Takeoff
        let mut start_json = vol_json["start"].clone();
        let start_str = start_json
            .take_string()
            .unwrap_or_else(|| "00h00".to_string())
            .clone();
        let takeoff = NaiveTime::parse_from_str(&start_str, "%Hh%M").unwrap();
        // Landing
        let stop_json = vol_json["stop"].clone();
        let stop_str = match stop_json {
            json::JsonValue::Short(short) => short.as_str().to_string(),
            _ => "00h00".to_string(),
        };
        let landing = NaiveTime::parse_from_str(stop_str.as_str(), "%Hh%M").unwrap();
        // TakeoffCode
        let mut takeoff_machine = "".to_string();
        let takeoff_code = if vol_json["tow"] == JsonValue::Null {
            "T"
        } else {
            let vol_remorqueur =
                vols_json[vol_json["tow"].clone().as_u8().unwrap() as usize].clone();
            let numero_immat_remorqueur = vol_remorqueur["device"].as_u8().unwrap() as usize;
            takeoff_machine = appareils_ogn[numero_immat_remorqueur]
                .immatriculation
                .clone();
            "R"
        }
        .to_string();

        vols.push(Flight {
            ogn_nb: index as i32,
            takeoff_code,
            takeoff_machine,
            takeoff_machine_pilot: "".to_string(),
            glider: immatriculation,
            flight_code: "".to_string(),
            pilot1: "".to_string(),
            pilot2: "".to_string(),
            takeoff,
            landing,
        });
    }
    Ok(vols)
}

/// Synchronizes the server requesting OGN latest data.
pub async fn synchronisation_ogn(
    flightlog_arc: Arc<Mutex<FlightLog>>,
    oaci: &String,
    context: &Context,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut flightlog_lock = flightlog_arc.lock().unwrap();
    let _ = flightlog_lock.update_ogn(&oaci, context);
    drop(flightlog_lock);
    return Ok(());
}
