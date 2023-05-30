use chrono::prelude::*;
use json::JsonValue::Array;
use std::fs;

#[derive(Clone, PartialEq)]
pub struct Vol {
    pub numero_ogn: i32,
    pub code_decollage: String,
    pub machine_decollage: String,
    pub decolleur: String,
    pub aeronef: String,
    pub code_vol: String,
    pub pilote1: String,
    pub pilote2: String,
    pub decollage: NaiveTime,
    pub atterissage: NaiveTime,
}

impl Vol {
    pub fn to_json(self: &Self) -> String {
        let vol = json::object! {
            numero_ogn: self.numero_ogn,
            code_decollage: *self.code_decollage,
            machine_decollage: *self.machine_decollage,
            decolleur: *self.decolleur,
            aeronef: *self.aeronef,
            code_vol: *self.code_vol,
            pilote1: *self.pilote1,
            pilote2: *self.pilote2,
            decollage: *self.decollage.format("%H:%M").to_string(),
            atterissage: *self.atterissage.format("%H:%M").to_string(),
        };
        vol.dump()
    }
}

pub struct Appareil {
    pub modele: String,
    pub categorie: u8,
    pub immatriculation: String,
}

pub fn liste_immatriculations() -> Vec<String> {
    let contenu_fichier = fs::read_to_string("./parametres/immatriculations.json")
        .expect("Probleme lors de la leture du fichier");
    let fichier_parse = json::parse(contenu_fichier.as_str()).unwrap();
    let immatriculations_json = match fichier_parse {
        Array(vecteur) => vecteur,
        _ => {
            eprintln!("immatriculations.json n'est pas un tableau");
            Vec::new()
        }
    };
    let mut immatriculations = Vec::new();
    for immatriculation_json in immatriculations_json {
        match immatriculation_json {
            json::JsonValue::Short(immatriculation) => {
                immatriculations.push(immatriculation.as_str().to_string());
            }
            _ => {
                eprintln!("{} n'est pas de type short", immatriculation_json);
            }
        }
    }
    immatriculations
}

pub fn ajouter_requete(mut requetes_en_cours: Vec<Client>, adresse: String) {
    //println!("+1 connection : {}", adresse.clone());
    let mut adresse_existe: bool = false;
    for mut client in requetes_en_cours.clone() {
        if client.adresse == adresse {
            if client.requetes_en_cours < 10 {
                client.requetes_en_cours += 1;
                adresse_existe = true;
            } else {
                println!("pas plus de requêtes pour {}", adresse);
            }
        }
        if adresse_existe == false {
            requetes_en_cours.push(Client {
                adresse: adresse.to_string(),
                requetes_en_cours: 1,
            });
        }
    }
}

pub fn enlever_requete(mut requetes_en_cours: Vec<Client>, adresse: String) {
    //println!("-1 connection : {}", adresse.clone());
    for mut client in requetes_en_cours.clone() {
        if client.adresse == adresse {
            if client.requetes_en_cours != 1 {
                client.requetes_en_cours -= 1;
            } else {
                let index = requetes_en_cours.iter().position(|x| *x == client).unwrap();
                requetes_en_cours.remove(index);
            }
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct Client {
    adresse: String,
    requetes_en_cours: i32,
}

pub struct MiseAJour {
    numero_vol: u8,
    champ_mis_a_jour: String,
    nouvelle_valeur: String,
}

impl MiseAJour {
    pub fn new() -> Self {
        MiseAJour {
            numero_vol: u8::default(), //numero du vol **OGN**
            champ_mis_a_jour: String::default(),
            nouvelle_valeur: String::default()
        }
    }
    
    pub fn parse(self: &mut Self, texte_json: json::JsonValue) -> Result<(), String> {
        match texte_json {
            json::JsonValue::Object(objet) => {
                self.numero_vol = objet["numero_vol"].as_u8().unwrap_or_else(|| {
                    eprintln!("pas de numero de vol dans la requete");
                    0
                });

                self.champ_mis_a_jour = objet["champ_mis_a_jour"]
                    .as_str()
                    .unwrap_or_else(|| {
                        eprintln!("pas le bon champ pour la nouvelle valeur");
                        ""
                    })
                    .to_string();

                self.nouvelle_valeur = objet["nouvelle_valeur"]
                    .as_str()
                    .unwrap_or_else(|| {
                        eprintln!("pas la bonne valeur pour la nouvelle valeur");
                        ""
                    })
                    .to_string();
            }
            _ => {
                eprintln!("pas un objet json");
            }
        };
        Ok(())
    }
}

// on crée une fonction pour mettre la mise à jour dans le vecteur Vols du jour
fn mise_a_jour(mut vols: Vec<Vol>, mise_a_jour: MiseAJour) {
    for mut vol in vols {
        if vol.numero_ogn == mise_a_jour.numero_vol as i32 {
            match mise_a_jour.champ_mis_a_jour.clone().as_str() {
                "code_decollage"    => vol.code_decollage    = mise_a_jour.nouvelle_valeur.clone(),
                "machine_decollage" => vol.machine_decollage = mise_a_jour.nouvelle_valeur.clone(), 
                "decolleur"         => vol.decolleur         = mise_a_jour.nouvelle_valeur.clone(), 
                "aeronef"           => vol.aeronef           = mise_a_jour.nouvelle_valeur.clone(), 
                "code_vol"          => vol.code_vol          = mise_a_jour.nouvelle_valeur.clone(),
                "pilote1"           => vol.pilote1           = mise_a_jour.nouvelle_valeur.clone(), 
                "pilote2"           => vol.pilote2           = mise_a_jour.nouvelle_valeur.clone(), 
                "decollage" => {
                    vol.decollage = NaiveTime::parse_from_str(format!("{}", mise_a_jour.nouvelle_valeur.clone()).as_str(), "%Hh%M").unwrap();
                },
                "atterissage" => {
                    vol.atterissage = NaiveTime::parse_from_str(format!("{}", mise_a_jour.nouvelle_valeur.clone()).as_str(), "%Hh%M").unwrap();
                },
                _ => {
                    eprintln!("Requète de mise a jour mauvaise.");
                }
            }
        }
    }
}