use std::fs;
use json::JsonValue::Array;
use chrono::prelude::*;

pub struct Vol {
    pub numero_ogn: i32,
    pub planeur: String,
    pub decollage: NaiveTime,
    pub atterissage: NaiveTime,
}

impl Vol {
    fn default() -> Self {
        Vol {
            numero_ogn: 0,
            planeur: "X-XXXX".to_string(),
            decollage: NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            atterissage: NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        }
    }

    pub fn to_json(self: &Self) -> String {
        let vol = json::object!{
            numero_ogn: self.numero_ogn,
            planeur: *self.planeur,
            decollage: *self.decollage.format("%Hh%M").to_string(),
            atterissage: *self.atterissage.format("%Hh%M").to_string(),
        };
        vol.dump()
    }
}

pub struct Appareil {
    pub modele: String,
    pub categorie: u8,
    pub immatriculation: String,
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
            json::JsonValue::Short(immatriculation) => {
                immatriculations.push(immatriculation.as_str().to_string());
            },
            _ => {
                eprintln!("{} n'est pas de type short", immatriculation_json);
            }
        }
    }
    immatriculations
}