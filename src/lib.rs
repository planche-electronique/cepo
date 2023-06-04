use std::fs;

pub use crate::vol::Vol;
pub use crate::vol::Json;
mod vol;

pub use crate::client::{VariationRequete, Client};
mod client;

pub use crate::mise_a_jour::{MiseAJour, MettreAJour};
mod mise_a_jour;


pub struct Appareil {
    pub modele: String,
    pub categorie: u8,
    pub immatriculation: String,
}

pub fn liste_immatriculations() -> Vec<String> {
    let contenu_fichier = fs::read_to_string("./parametres/immatriculations.json")
        .expect("Probleme lors de la leture du fichier");
    let fichier_parse = json::parse(contenu_fichier.as_str()).unwrap();
    let iter_fichier = fichier_parse.members();
    let mut immatriculations = Vec::new();
    for valeur_json in iter_fichier {
        immatriculations.push(valeur_json.as_str().unwrap().to_string());
    }

    immatriculations
}


pub fn nom_fichier_date(nombre: i32) -> String {
    let nombre_str: String;
    if nombre > 9 {
        nombre_str = nombre.to_string();
    } else {
        nombre_str = format!("0{}", nombre.to_string());
    } 
    nombre_str
}