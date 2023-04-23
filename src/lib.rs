use std::fs;
use json::JsonValue::Array;
use chrono::prelude::*;
use chrono::format::strftime::StrftimeItems;

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

async fn requete_ogn(date: NaiveDate) -> String {
    let airfield_code = "LFLE";
    let reponse = reqwest::get(format!("http://flightbook.glidernet.org/api/logbook/{}/{}", airfield_code, date.format("%Y-%m-%d").to_sring())).await.unwrap();
    let corps = reponse.text().await.unwrap();
    corps
}

fn traitement_requete_ogn(date: NaiveDate)

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