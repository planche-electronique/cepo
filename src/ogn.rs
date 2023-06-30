use crate::planche::Planche;
use crate::vol::Vol;
use crate::Appareil;
use chrono::prelude::*;
use json::JsonValue;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;

pub fn thread_ogn(planche: Arc<Mutex<Planche>>) {
    let date = NaiveDate::from_ymd_opt(2023, 04, 25).unwrap();
    let planche_lock = planche.lock().unwrap();
    let mut ancienne_planche = (*planche_lock).clone();
    drop(planche_lock);
    //on teste les égalités et on remplace si besoin
    let requete = requete_ogn(date);
    match requete {
        Ok(requete_developpee) => {
            let nouvelle_planche = traitement_requete_ogn(requete_developpee, date);
            for nouveau_vol in nouvelle_planche.vols.clone() {
                let mut existe = false;
                for ancien_vol in &mut ancienne_planche.vols {
                    // si on est sur le meme vol
                    if nouveau_vol.numero_ogn == ancien_vol.numero_ogn {
                        existe = true;
                        let heure_default = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
                        //teste les différentes valeurs qui peuvent être mises a jour
                        if ancien_vol.decollage == heure_default {
                            (*ancien_vol).decollage = nouveau_vol.decollage;
                        }
                        if ancien_vol.atterissage == heure_default {
                            (*ancien_vol).atterissage = nouveau_vol.atterissage;
                        }
                    }
                }
                if !existe {
                    ancienne_planche.vols.push(nouveau_vol);
                }
            }

            let mut planche_lock = planche.lock().unwrap();
            *planche_lock = ancienne_planche.clone();
            drop(planche_lock);
            ancienne_planche.enregistrer();
            // 5 minutes
            thread::sleep(time::Duration::from_millis(300000));
        }
        Err(_) => {
            println!("Impossible de se connecter àl'A.P.I. de O.G.N. Veuillez vérifier votre connection internet.");
            thread::sleep(time::Duration::from_millis(30000));
        }
    }
}

pub fn requete_ogn(date: NaiveDate) -> Result<String, reqwest::Error> {
    let airfield_code = "LFLE";
    log::info!(
        "Requete à http://flightbook.glidernet.org/api/logbook/{}/{}",
        airfield_code,
        date.format("%Y-%m-%d").to_string()
    );
    let reponse = reqwest::blocking::get(format!(
        "http://flightbook.glidernet.org/api/logbook/{}/{}",
        airfield_code,
        date.format("%Y-%m-%d").to_string()
    ));
    match reponse {
        Ok(reponse_developpee) => {
            let corps = reponse_developpee.text().unwrap();
            Ok(corps)
        }
        Err(erreur) => Err(erreur),
    }
}

pub fn traitement_requete_ogn(requete: String, date: NaiveDate) -> Planche {
    let requete_parse = json::parse(requete.as_str()).unwrap();
    log::info!("Traitement de la requete {}", requete.clone());

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
    let mut index = 0;
    let immatriculations = crate::paramtres_liste_depuis_json("immatriculations.json");
    for vol_json in vols_json.clone() {
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
        //decoollage
        let mut start_json = vol_json["start"].clone();
        let start_str = start_json
            .take_string()
            .unwrap_or_else(|| "00h00".to_string())
            .clone();
        let decollage =
            NaiveTime::parse_from_str(format!("{}", start_str).as_str(), "%Hh%M").unwrap();
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
            numero_ogn: index,
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
    Planche { vols, date }
}
