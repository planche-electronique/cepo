use chrono::prelude::*;
use json::JsonValue::Array;
use serveur::{Appareil, Vol};
use std::fs;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;

pub fn thread_ogn(vols: Arc<Mutex<Vec<Vol>>>) {
    let date = NaiveDate::from_ymd_opt(2023, 04, 25).unwrap();
    let vols_lock = vols.lock().unwrap();
    let mut anciens_vols = (*vols_lock).clone();
    drop(vols_lock);
    //on teste les égalités
    let nouveaux_vols = traitement_requete_ogn(requete_ogn(date));
    for ancien_vol in &mut anciens_vols {
        for nouveau_vol in nouveaux_vols.clone() {
            let heure_default = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
            if ancien_vol.decollage == heure_default {
                ancien_vol.decollage = nouveau_vol.decollage;
            }
            if ancien_vol.atterissage == heure_default {
                ancien_vol.atterissage = nouveau_vol.atterissage;
            }
        }
    }

    let mut vols_lock = vols.lock().unwrap();
    *vols_lock = anciens_vols;
    enregistrer_vols(vols_lock.to_vec(), date);
    drop(vols_lock);
    thread::sleep(time::Duration::from_millis(300000)); // 5 minutes
}

pub fn requete_ogn(date: NaiveDate) -> String {
    let airfield_code = "LFLE";
    let reponse = reqwest::blocking::get(format!(
        "http://flightbook.glidernet.org/api/logbook/{}/{}",
        airfield_code,
        date.format("%Y-%m-%d").to_string()
    ))
    .unwrap();
    let corps = reponse.text().unwrap();
    corps
}

fn traitement_requete_ogn(requete: String) -> Vec<Vol> {
    let requete_parse = json::parse(requete.as_str()).unwrap();
    let devices = requete_parse["devices"].clone();
    let mut appareils_ogn: Vec<Appareil> = Vec::new();
    let tableau_devices = match devices {
        Array(appareils_json) => appareils_json,
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
            modele: modele,
            categorie: categorie,
            immatriculation: immatriculation,
        };
        appareils_ogn.push(appareil_actuel);
    }

    let mut vols: Vec<Vol> = Vec::new();
    let flights = requete_parse["flights"].clone();
    let vols_json = match flights {
        Array(vols_json) => vols_json,
        _ => {
            eprintln!("n'est pas un tableau");
            Vec::new()
        }
    };
    let mut index = 1;
    for vol_json in vols_json {
        let mut start_json = vol_json["start"].clone();
        let start_str = start_json
            .take_string()
            .unwrap_or_else(|| "00h00".to_string())
            .clone();
        let decollage =
            NaiveTime::parse_from_str(format!("{}", start_str).as_str(), "%Hh%M").unwrap();

        let stop_json = vol_json["stop"].clone();
        let stop_str = match stop_json {
            json::JsonValue::Short(short) => short.as_str().to_string(),
            _ => "00h00".to_string(),
        };
        let atterissage = NaiveTime::parse_from_str(stop_str.as_str(), "%Hh%M").unwrap();

        let device = vol_json["device"].clone();
        let device_number = device.as_u8().unwrap() as usize;
        let immatriculation = appareils_ogn[device_number].immatriculation.clone();

        vols.push(Vol {
            numero_ogn: index,
            code_decollage: "".to_string(),
            machine_decollage: "".to_string(),
            decolleur: "".to_string(),
            aeronef: immatriculation,
            code_vol: "".to_string(),
            pilote1: "".to_string(),
            pilote2: "".to_string(),
            decollage: decollage,
            atterissage: atterissage,
        });
        index += 1;

        let immatriculations = serveur::liste_immatriculations();
        for vol in vols.clone() {
            if !(immatriculations.iter().any(|immat| *immat == vol.aeronef)) {
                //si l'immat n'est pas dans la liste
                let index = vols.iter().position(|x| *x == vol).unwrap();
                vols.remove(index); // on l'enleve
            }
        }
    }
    vols
}

fn enregistrer_vols(vols: Vec<Vol>, date: NaiveDate) {
    let annee = date.format("%Y").to_string();
    let mois = date.format("%m").to_string();
    let jour = date.format("%d").to_string();

    creer_chemin_jour(annee.clone(), mois.clone(), jour.clone());

    let mut vols_json = Vec::new();
    for vol in vols {
        vols_json.push(vol.to_json());
    }

    let mut index = 1;
    for vol_json in vols_json {
        let chemin = format!(
            "./dossier_de_travail/{}/{}/{}/{}.json",
            annee, mois, jour, index
        );
        let fichier = fs::read_to_string(chemin.clone()).unwrap_or_else(|err| {
            println!(
                "fichier numero {} introuvable ou non ouvrable : {}",
                index,
                err.to_string()
            );
            "".to_string()
        });

        if fichier != vol_json {
            fs::write(chemin, vol_json).expect("impossible d'ecrire le fichier");
        }
        index += 1;
    }
}

fn creer_chemin_jour(annee: String, mois: String, jour: String) {
    let chemins = fs::read_dir("./dossier_de_travail").unwrap();
    let mut annee_existe = false;
    for chemin in chemins {
        let chemin_dossier = chemin.unwrap().path().to_str().unwrap().to_string();
        if chemin_dossier == format!("./dossier_de_travail\\{}", annee) {
            annee_existe = true;
        }
    }
    if annee_existe == false {
        fs::create_dir(format!("./dossier_de_travail\\{}", annee)).unwrap();
    }

    let chemins = fs::read_dir(format!("./dossier_de_travail\\{}", annee)).unwrap();
    let mut mois_existe = false;
    for chemin in chemins {
        let chemin_dossier = chemin.unwrap().path().to_str().unwrap().to_string();
        if chemin_dossier == format!("./dossier_de_travail\\{}\\{}", annee, mois) {
            mois_existe = true;
        }
    }
    if mois_existe == false {
        fs::create_dir(format!("./dossier_de_travail\\{}\\{}", annee, mois)).unwrap();
    }

    let chemins = fs::read_dir(format!("./dossier_de_travail\\{}\\{}", annee, mois)).unwrap();
    let mut jour_existe = false;
    for chemin in chemins {
        let chemin_dossier = chemin.unwrap().path().to_str().unwrap().to_string();
        if chemin_dossier == format!("./dossier_de_travail\\{}\\{}\\{}", annee, mois, jour) {
            jour_existe = true;
        }
    }
    if jour_existe == false {
        fs::create_dir(format!("./dossier_de_travail\\{}/{}/{}", annee, mois, jour)).unwrap();
    }
}
