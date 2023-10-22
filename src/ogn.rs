//! Pour gérer les requêtes à OGN.

use crate::vol::{MettreAJour, Vol};
use crate::{Appareil, ActifServeur};
use chrono::prelude::*;
use json::JsonValue;
use log;
use std::fs;

/// Retourne les vols récupérés par requête GET à OGN.
pub async fn vols_ogn(date: NaiveDate, airfield_oaci: String) -> Result<Vec<Vol>, hyper::Error> {
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
    let mut appareils_ogn: Vec<Appareil> = Vec::new();
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

        let appareil_actuel = Appareil {
            modele,
            categorie,
            immatriculation,
        };
        appareils_ogn.push(appareil_actuel);
    }

    /* ic on s'occupe de lister les vols et d'attribuer les
    immatriculations etc a chaque vol */

    let mut vols: Vec<Vol> = Vec::new();
    let flights = requete_parse["flights"].clone();
    let vols_json = match flights {
        JsonValue::Array(vols_json) => vols_json,
        _ => {
            eprintln!("La requete ogn n'a pas fourni un tableau.");
            Vec::new()
        }
    };

    let contenu_fichier = fs::read_to_string("../planche/infos.json").unwrap();
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
        let decollage = NaiveTime::parse_from_str(&start_str, "%Hh%M").unwrap();
        //atterissage
        let stop_json = vol_json["stop"].clone();
        let stop_str = match stop_json {
            json::JsonValue::Short(short) => short.as_str().to_string(),
            _ => "00h00".to_string(),
        };
        let atterissage = NaiveTime::parse_from_str(stop_str.as_str(), "%Hh%M").unwrap();
        //code_decollage
        let mut machine_decollage = "".to_string();
        let code_decollage = if vol_json["tow"] == JsonValue::Null {
            "T"
        } else {
            let vol_remorqueur =
                vols_json[vol_json["tow"].clone().as_u8().unwrap() as usize].clone();
            let numero_immat_remorqueur = vol_remorqueur["device"].as_u8().unwrap() as usize;
            machine_decollage = appareils_ogn[numero_immat_remorqueur]
                .immatriculation
                .clone();
            "R"
        }
        .to_string();

        vols.push(Vol {
            numero_ogn: index as i32,
            code_decollage,
            machine_decollage,
            decolleur: "".to_string(),
            aeronef: immatriculation,
            code_vol: "".to_string(),
            pilote1: "".to_string(),
            pilote2: "".to_string(),
            decollage,
            atterissage,
        });
    }
    Ok(vols)
}

/// Synchronise le serveur notamment en faisant une requête à OGN et en mettant à jour la planche du jour.
pub async fn synchronisation_ogn(
    actif_serveur: &ActifServeur,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let date = chrono::Local::now().date_naive();
    let vols_ogn = vols_ogn(date, actif_serveur.configuration.oaci.clone()).await?;
    let planche_lock = actif_serveur.planche.lock().unwrap();
    let mut ancienne_planche = (*planche_lock).clone();
    drop(planche_lock);
    //on teste les égalités et on remplace si besoin
    ancienne_planche.vols.mettre_a_jour(vols_ogn);

    let mut planche_lock = actif_serveur.planche.lock().unwrap();
    *planche_lock = ancienne_planche.clone();
    drop(planche_lock);
    ancienne_planche.enregistrer();
    Ok(())
}
