use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use crate::planche::{Planche, MiseAJour};
use crate::client::Client;

pub mod client;
pub mod ogn;
pub mod planche;
pub mod vol;

pub struct Appareil {
    pub modele: String,
    pub categorie: u8,
    pub immatriculation: String,
}

pub fn nom_fichier_date(nombre: i32) -> String {
    if nombre > 9 {
        nombre.to_string()
    } else {
        format!("0{}", nombre)
    }
}

pub fn creer_chemin_jour(annee: i32, mois: u32, jour: u32) {
    let jour_str = nom_fichier_date(jour as i32);
    let mois_str = nom_fichier_date(mois as i32);

    if !(Path::new(format!("../site/dossier_de_travail/{}", annee).as_str()).exists()) {
        fs::create_dir(format!("../site/dossier_de_travail/{}", annee)).unwrap();
    }

    if !(Path::new(format!("../site/dossier_de_travail/{}/{}", annee, mois_str).as_str()).exists())
    {
        fs::create_dir(format!("../site/dossier_de_travail/{}/{}", annee, mois_str)).unwrap();
    }

    if !(Path::new(
        format!(
            "../site/dossier_de_travail/{}/{}/{}",
            annee, mois_str, jour_str
        )
        .as_str(),
    )
    .exists())
    {
        fs::create_dir(format!(
            "../site/dossier_de_travail/{}/{}/{}",
            annee, mois_str, jour_str
        ))
        .unwrap();
        log::info!("Création du chemin {}/{}/{}", annee, mois_str, jour_str);
    }
}


#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Configuration {
    pub oaci: String,
    pub f_synchronisation_secs: i32,
    pub port: i32,
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

#[derive(Clone)]
pub struct ActifServeur {
    pub configuration: Configuration,
    pub planche: Arc<Mutex<Planche>>,
    pub majs: Arc<Mutex<Vec<MiseAJour>>>,
    pub requetes_en_cours: Arc<Mutex<Vec<Client>>>,
}
