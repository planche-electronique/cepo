use std::fs;
use std::path::Path;

pub mod client;
pub mod ogn;
pub mod planche;
pub mod vol;

pub struct Appareil {
    pub modele: String,
    pub categorie: u8,
    pub immatriculation: String,
}

pub fn parametres_liste_depuis_json(fichier: &str) -> Vec<String> {
    log::info!("Lecture de la liste de paramètres {}", fichier);
    let contenu_fichier = fs::read_to_string(format!("../site/{}", fichier)).expect(&format!(
        "Probleme lors de la leture du fichier : {}",
        fichier
    ));
    let fichier_parse = json::parse(contenu_fichier.as_str()).unwrap();
    let iter_fichier = fichier_parse.members();
    let mut elements = Vec::new();
    for valeur_json in iter_fichier {
        elements.push(valeur_json.as_str().unwrap().to_string());
    }
    elements
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
