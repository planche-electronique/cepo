//! Pour gérer les requêtes à OGN.

use crate::flightlog::Storage;
use crate::flight::Update;
use crate::{Context, Aircraft};
use brick_ogn::flight::Flight;
use chrono::prelude::*;
use json::JsonValue;
use log;
use std::fs;

/// Retourne les vols récupérés par requête GET à OGN.
pub async fn ogn_flights(date: NaiveDate, airfield_oaci: String) -> Result<Vec<Flight>, hyper::Error> {
    log::info!(
        "Requete à http://flightbook.glidernet.org/api/logbook/{}/{}",
        airfield_oaci,
        date.format("%Y-%m-%d").to_string()
    );
    let client = hyper::Client::new();
    let chemin = format!(
        "http://flightbook.glidernet.org/api/logbook/{}/{}",
        airfield_oaci,
        date.format("%Y-%m-%d")
    )
    .parse::<hyper::Uri>()
    .unwrap();
    let reponse = client.get(chemin).await?;
    let bytes = hyper::body::to_bytes(reponse.into_body()).await?;
    let corps_str = std::str::from_utf8(&bytes).unwrap().to_string();

    let requete_parse = json::parse(corps_str.as_str()).unwrap();
    log::info!("Traitement de la requete.");

    /* ogn repere les aéronefs d'un jour en les listants et leur attribuant un id,
    nous devons donc faire un lien entre l'immatriculation et le numero
    d'un aeronef */
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

        let categorie_json = appareil["aircraft_type"].clone();
        let categorie = categorie_json.as_u8().unwrap();

        let immatriculation_json = appareil["registration"].clone();
        let immatriculation = immatriculation_json
            .as_str()
            .unwrap_or_default()
            .to_string();

        let appareil_actuel = Aircraft {
            modele,
            categorie,
            immatriculation,
        };
        appareils_ogn.push(appareil_actuel);
    }

    /* ic on s'occupe de lister les vols et d'attribuer les
    immatriculations etc a chaque vol */

    let mut vols: Vec<Flight> = Vec::new();
    let flights = requete_parse["flights"].clone();
    let vols_json = match flights {
        JsonValue::Array(vols_json) => vols_json,
        _ => {
            eprintln!("La requete ogn n'a pas fourni un tableau.");
            Vec::new()
        }
    };

    let contenu_fichier = fs::read_to_string(crate::data_dir().as_path().join("infos.json"))
        .unwrap_or_else(|_| "{}".to_string());
    let fichier_parse = json::parse(contenu_fichier.as_str()).unwrap();
    let immatriculations_json = &fichier_parse["immatriculations"];
    let iter_fichier = immatriculations_json.members();
    let mut immatriculations = Vec::new();
    for valeur_json in iter_fichier {
        immatriculations.push(valeur_json.as_str().unwrap().to_string());
    }
    for (mut index, vol_json) in vols_json.clone().into_iter().enumerate() {
        index += 1;

        // on recupere tous les champs nécessaires
        let device = vol_json["device"].clone();
        let device_number = device.as_u8().unwrap() as usize;
        let immatriculation = appareils_ogn[device_number].immatriculation.clone();
        if !(immatriculations
            .iter()
            .any(|immat| *immat == immatriculation.clone()))
        {
            //si l'immat n'est pas dans la liste, on ne la prend pas en compte
            continue;
        }
        //decollage
        let mut start_json = vol_json["start"].clone();
        let start_str = start_json
            .take_string()
            .unwrap_or_else(|| "00h00".to_string())
            .clone();
        let takeoff = NaiveTime::parse_from_str(&start_str, "%Hh%M").unwrap();
        //atterissage
        let stop_json = vol_json["stop"].clone();
        let stop_str = match stop_json {
            json::JsonValue::Short(short) => short.as_str().to_string(),
            _ => "00h00".to_string(),
        };
        let landing = NaiveTime::parse_from_str(stop_str.as_str(), "%Hh%M").unwrap();
        //code_decollage
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
    context: &Context,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let date = chrono::Local::now().date_naive();
    let flights_ogn = ogn_flights(date, context.configuration.oaci.clone()).await?;
    let flightlog_lock =  context.flightlog.lock().unwrap();
    let mut old_flightlog = (*flightlog_lock).clone();
    drop(flightlog_lock);
    // testing equality and replacing if needed
    old_flightlog.flights.update(flights_ogn);

    let mut flightlog_lock = context.flightlog.lock().unwrap();
    *flightlog_lock = old_flightlog.clone();
    drop(flightlog_lock);
    old_flightlog.save();
    Ok(())
}
