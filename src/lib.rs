use std::fs;
use json::JsonValue::{Short, Null, Number, Boolean, Object, Array};
use chrono::prelude::*;


struct Vol {
    numero: i32,
    planeur: String,
    pilote_1: String,
    pilote_2: String,
    code_vol: String,
    code_deco: String,
    machine_deco: String,
    heure_decolage: u8,
    minute_decolage: u8,
    heure_atterissage: u8,
    minute_atterissage: u8,
}

struct Appareil {
    modele: String,
    categorie: u8,
    immatriculation: String,
}

pub fn requete_ogn(date: NaiveDate) -> String {
    let airfield_code = "LFLE";
    let reponse = reqwest::blocking::get(format!("http://flightbook.glidernet.org/api/logbook/{}/{}", airfield_code, date.format("%Y-%m-%d").to_string())).unwrap();
    let corps = reponse.text().unwrap();
    corps
}

pub fn traitement_requete_ogn(date: NaiveDate, requete: String) {
    println!("{}", requete);
    let requete_parse = json::parse(requete.as_str()).unwrap();
    let devices = requete_parse["devices"].clone();
    let mut appareils_ogn: Vec<Appareil> = Vec::new();
    let tableau_devices = match devices {
        Array(appareils_json) => appareils_json,
        _ => {
            eprintln!("devices n'est pas un tableau");
            Vec::new()
        },
    };
    for appareil in tableau_devices {
        let modele_json = appareil["aircraft"].clone();
        let modele = modele_json.as_str().unwrap().to_string();
        
        let categorie_json = appareil["aircraft_type"].clone();
        let categorie = categorie_json.as_u8().unwrap();
        
        let immatriculation_json = appareil["registration"].clone();
        let immatriculation = immatriculation_json.as_str().unwrap().to_string();
        

        let appareil_actuel = Appareil {
            modele: modele,
            categorie: categorie,
            immatriculation: immatriculation,
        };
        appareils_ogn.push(appareil_actuel);
    }
    //on ne garde que les appareils de la liste d'immatriculations
    let mut appareils_cibles: Vec<Appareil> = Vec::new();
    for immatriculation in liste_immatriculations() {
        for appareil in appareils_ogn {
            if immatriculation == appareil.immatriculation {
                appareils_cibles.push(appareil);
            }
        }
    }

    //on crée un vecteur de struct Vol
    /* on itere sur le tableau de vols (nommé "flights") retourné par l'api
    infos utiles:
    "start"
    "stop"
    "device"
    "towing" auquel ca "tow" aussi
    */
}

fn liste_immatriculations() -> Vec<String> {
    let contenu_fichier = fs::read_to_string("immatriculations.json")
        .expect("Probleme lors de la leture du fichier");
    let fichier_parse = json::parse(contenu_fichier.as_str()).unwrap();
    let immatriculations_json = match fichier_parse {   
        Array(vecteur) => {
            vecteur
        },
        _ => {
            eprintln!("immatriculations.json n'est pas un tableau");
            Vec::new()
        },
    };
    let mut immatriculations = Vec::new();
    for immatriculation_json in immatriculations_json {
        match immatriculation_json {
            json::JsonValue::String(immatriculation) => {
                immatriculations.push(immatriculation);
            },
            _ => {
                eprintln!("{} n'est pas de type string", immatriculation_json);
            }
        }
    }
    immatriculations
}