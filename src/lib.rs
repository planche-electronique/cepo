#![warn(missing_docs)]


//! Utilisation facile et rapide des donnéess d'OGN pour enregistrer décollages et atterissages de vols en planeur.
//! Il vous suffit de fournir les données sous forme de tableau des pilotes, planeurs, remorqueurs et treuilleurs de votre club.
//! La planche fonctionnera alors.

use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use crate::planche::{Planche, MiseAJour};
use crate::client::Client;

pub mod client;
pub mod ogn;
pub mod planche;
pub mod vol;

/// Représentation d'un aéronef.
pub struct Appareil {
    /// Le modèle/type de l'aéronef.
    pub modele: String,
    /// La catégorie de cet aéronef (avion, planeur...).
    pub categorie: u8,
    /// L'immatriculation de cet aéronef(F-CMOI...).
    pub immatriculation: String,
}

/// Ajoute un 0 devant le nombre s'il est inférieur à 10 pour avoir des strings à 2 chiffres et à longueur fixe.
/// # Exemple
/// ```
/// use serveur::nom_fichier_date;
/// assert_eq!(nom_fichier_date(2), String::from("02"));
/// assert_eq!(nom_fichier_date(20), String::from("20"));
/// ```
pub fn nom_fichier_date(nombre: i32) -> String {
    if nombre > 9 {
        nombre.to_string()
    } else {
        format!("0{}", nombre)
    }
}

/// Permet de créer le chemin du jour à "../site/dossier_de_travail/annee/mois/jour".
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

/// Permet de stocker et partager la configuration du serveur. Elle est chargée grâce à 
/// [confy](https://crates.io/crates/confy). Elle a une valeur par défaut qui est écrite si le
/// fichier de confiuration est inexistant.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Configuration {
    /// Le code OACI de l'aéroport dont les vols vont être loggés.
    pub oaci: String,
    /// Le temps d'attente entre chaque requête au serveur OGN.
    pub f_synchronisation_secs: i32,
    /// Le port sur lequel le serveur va écouter les requêtes (7878 par défaut).
    pub port: i32,
    /// Le niveau de log à afficher dans le terminal ("info" par défaut). A choisir parmis "trace",
    /// "debug", "info", "warn", "error".
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

/// Suprerstructure du serveur. Elle permet de stocker la configuration et les structures de
/// données telles que la planche du jour, les requêtes en cours et les requêtes des 5 dernières minutes.
#[derive(Clone)]
pub struct ActifServeur {
    /// La configuration du serveur.
    pub configuration: Configuration,
    /// La planche du jour, en mémoire et partageable entre threads.
    pub planche: Arc<Mutex<Planche>>,
    /// Un vecteur de mise_a_jour pour alléger les requêtes des planches. Les mises à jour ne sont
    /// gardées que 5 minutes.
    pub majs: Arc<Mutex<Vec<MiseAJour>>>,
    /// Un vecteur qui permet de comptabiliser le nombre de requêtes en cours pour éviter les ddos.
    pub requetes_en_cours: Arc<Mutex<Vec<Client>>>,
}
