use chrono::prelude::*;
use crate::vol::Vol;

#[derive(Debug, PartialEq)]
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
            nouvelle_valeur: String::default(),
        }
    }

    pub fn parse(&mut self, texte_json: json::JsonValue) -> Result<(), String> {
        match texte_json {
            json::JsonValue::Object(objet) => {
                self.numero_vol = objet["numero_ogn"].as_u8().unwrap_or_else(|| {
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

pub trait MettreAJour {
    fn mettre_a_jour(&mut self, mise_a_jour: MiseAJour);
}

impl MettreAJour for Vec<Vol> {
    // on crée une fonction pour mettre la mise à jour dans le vecteur Vols du jour
    fn mettre_a_jour(&mut self, mise_a_jour: MiseAJour) {
        for mut vol in self {
            if vol.numero_ogn == mise_a_jour.numero_vol as i32 {
                match mise_a_jour.champ_mis_a_jour.clone().as_str() {
                    "code_decollage" => vol.code_decollage = mise_a_jour.nouvelle_valeur.clone(),
                    "machine_decollage" => vol.machine_decollage = mise_a_jour.nouvelle_valeur.clone(),
                    "decolleur" => vol.decolleur = mise_a_jour.nouvelle_valeur.clone(),
                    "aeronef" => vol.aeronef = mise_a_jour.nouvelle_valeur.clone(),
                    "code_vol" => vol.code_vol = mise_a_jour.nouvelle_valeur.clone(),
                    "pilote1" => vol.pilote1 = mise_a_jour.nouvelle_valeur.clone(),
                    "pilote2" => vol.pilote2 = mise_a_jour.nouvelle_valeur.clone(),
                    "decollage" => {
                        vol.decollage = NaiveTime::parse_from_str(
                            format!("{}", mise_a_jour.nouvelle_valeur.clone()).as_str(),
                            "%Hh%M",
                        )
                        .unwrap();
                    }
                    "atterissage" => {
                        vol.atterissage = NaiveTime::parse_from_str(
                            format!("{}", mise_a_jour.nouvelle_valeur.clone()).as_str(),
                            "%Hh%M",
                        )
                        .unwrap();
                    }
                    _ => {
                        eprintln!("Requète de mise a jour mauvaise.");
                    }
                }
            }
        }
        
    }            
}

mod tests {


    #[test]
    fn mise_a_jour_parse_test() {
        use crate::MiseAJour;
        
        let mise_a_jour_declaree = MiseAJour {
            numero_vol: 1,
            champ_mis_a_jour: String::from("code_vol"),
            nouvelle_valeur: String::from("M")
        };
        
        let mut mise_a_jour_parse = MiseAJour::new();
        let _ = mise_a_jour_parse.parse(json::parse(" \
            { \
                \"numero_ogn\": 1, \
                \"champ_mis_a_jour\": \"code_vol\", \
                \"nouvelle_valeur\": \"M\" \
            }").unwrap());
        
        assert_eq!(mise_a_jour_declaree, mise_a_jour_parse)
    }
    
    #[test]
    fn mettre_a_jour_test() {
        use crate::{MiseAJour, MettreAJour};
        use crate::vol::Vol;
        use chrono::NaiveTime;
        
        let mut vols = Vec::new();
        vols.push(Vol::default());
        
        let mise_a_jour = MiseAJour {
            numero_vol: 1,
            champ_mis_a_jour: String::from("machine_decollage"),
            nouvelle_valeur: String::from("LUCIFER")
        };
        
        vols.mettre_a_jour(mise_a_jour);
        
        let vol_verif = Vol {
            numero_ogn: 1,
            code_decollage: String::from("T"),
            machine_decollage: String::from("LUCIFER"),
            decolleur: String::from("YDL"),
            aeronef: String::from("F-CERJ"),
            code_vol: String::from("S"),
            pilote1: String::from("Walt Disney"),
            pilote2: String::default(),
            decollage: NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
            atterissage: NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
        };
        assert_eq!(vols[0], vol_verif)
    }
}