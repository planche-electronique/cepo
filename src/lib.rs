use log;
use std::fs;

pub mod client;
pub mod ogn;
pub mod planche;
pub mod vol;

pub struct Appareil {
    pub modele: String,
    pub categorie: u8,
    pub immatriculation: String,
}

pub fn paramtres_liste_depuis_json(fichier: &str) -> Vec<String> {
    log::info!("Lecture de la liste de paramètres {}", fichier);
    let contenu_fichier = fs::read_to_string(format!("./parametres/{}", fichier))
        .expect(format!("Probleme lors de la leture du fichier : {}", fichier).as_str());
    let fichier_parse = json::parse(contenu_fichier.as_str()).unwrap();
    let iter_fichier = fichier_parse.members();
    let mut elements = Vec::new();
    for valeur_json in iter_fichier {
        elements.push(valeur_json.as_str().unwrap().to_string());
    }
    elements
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

pub fn creer_chemin_jour(annee: i32, mois: u32, jour: u32) {
    log::info!("Création du chemin {}/{}/{}", annee, mois, jour);
    let jour_str = nom_fichier_date(jour as i32);
    let mois_str = nom_fichier_date(mois as i32);

    let chemins = fs::read_dir("./dossier_de_travail").unwrap();
    let mut annee_existe = false;
    for chemin in chemins {
        let chemin_dossier = chemin.unwrap().path().to_str().unwrap().to_string();
        if chemin_dossier[21..25] == annee.to_string() {
            annee_existe = true;
        }
    }
    if annee_existe == false {
        fs::create_dir(format!("./dossier_de_travail/{}", annee)).unwrap();
    }

    let chemins = fs::read_dir(format!("./dossier_de_travail/{}", annee)).unwrap();
    let mut mois_existe = false;
    for chemin in chemins {
        let chemin_dossier = chemin.unwrap().path().to_str().unwrap().to_string();
        if chemin_dossier[26..28] == mois_str {
            mois_existe = true;
        }
    }
    if mois_existe == false {
        fs::create_dir(format!("./dossier_de_travail/{}/{}", annee, mois_str)).unwrap();
    }

    let chemins = fs::read_dir(format!("./dossier_de_travail/{}/{}", annee, mois_str)).unwrap();
    let mut jour_existe = false;
    for chemin in chemins {
        let chemin_dossier = chemin.unwrap().path().to_str().unwrap().to_string();
        if chemin_dossier[29..31] == jour_str {
            jour_existe = true;
        }
    }
    if jour_existe == false {
        fs::create_dir(format!(
            "./dossier_de_travail/{}/{}/{}",
            annee, mois_str, jour_str
        ))
        .unwrap();
    }
}
