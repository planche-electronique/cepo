use crate::ogn::{requete_ogn, traitement_requete_ogn};
use crate::vol::{Vol, VolJson};
use crate::{creer_chemin_jour, nom_fichier_date};
use chrono::{Datelike, NaiveDate, NaiveTime};
use std::fs;

#[derive(PartialEq, Debug, Clone)]
pub struct Planche {
    pub vols: Vec<Vol>,
    pub date: NaiveDate,
}

impl Planche {
    pub fn planche_du(date: NaiveDate) -> Planche {
        let annee = date.year();
        let mois = date.month();
        let jour = date.day();

        creer_chemin_jour(annee, mois, jour);

        //on récupère les données du vol même s'il n'y a pas d'informations
        let requete = requete_ogn(date);
        let planche_du_jour = traitement_requete_ogn(requete, date);
        planche_du_jour.enregistrer();

        let mois_str = nom_fichier_date(mois as i32);
        let jour_str = nom_fichier_date(jour as i32);

        let mut vols: Vec<Vol> = Vec::new();

        let fichiers = fs::read_dir(format!(
            "./dossier_de_travail/{}/{}/{}",
            annee, mois_str, jour_str
        ))
        .unwrap();

        for fichier in fichiers {
            let vol_json = fs::read_to_string(fichier.unwrap().path().to_str().unwrap()).unwrap();
            let vol = Vol::depuis_json(json::parse(vol_json.as_str()).unwrap());
            vols.push(vol);
        }
        Planche { date, vols }
    }

    pub fn enregistrer(&self) {
        let date = self.date;
        let vols = self.vols.clone();
        let annee = date.year();
        let mois = date.month();
        let jour = date.day();

        let jour_str = nom_fichier_date(jour as i32);
        let mois_str = nom_fichier_date(mois as i32);

        creer_chemin_jour(annee, mois, jour);

        let mut index = 1;
        for vol in vols {
            let index_str = nom_fichier_date(index);
            let chemin = format!(
                "./dossier_de_travail/{}/{}/{}/{}.json",
                annee, mois_str, jour_str, index_str
            );
            let mut fichier = String::new();
            if std::path::Path::new(chemin.clone().as_str()).exists() {
                fichier = fs::read_to_string(chemin.clone()).unwrap_or_else(|err| {
                    println!(
                        "fichier numero {} de chemin {} introuvable ou non ouvrable : {}",
                        index,
                        chemin.clone(),
                        err.to_string()
                    );
                    "".to_string()
                });
            }

            if fichier != vol.vers_json() {
                fs::write(chemin, vol.vers_json()).expect("impossible d'ecrire le fichier");
            }
            index += 1;
        }
    }

    pub fn new() -> Self {
        Planche {
            vols: Vec::new(),
            date: NaiveDate::default(),
        }
    }

    pub fn vers_json(self) -> String {
        let vols_json = self.vols.vers_json();
        let date_json = self.date.format("%Y/%m/%d").to_string();
        let mut json = String::new();
        json.push_str("{ \"date\": \"");
        json.push_str(&date_json);
        json.push_str("\",\n\"vols\" : ");
        json.push_str(&vols_json);
        json.push_str("}");
        return json;
    }
}

pub trait MettreAJour {
    fn mettre_a_jour(&mut self, mise_a_jour: MiseAJour);
}

impl MettreAJour for Planche {
    // on crée une fonction pour mettre la mise à jour dans le vecteur Vols du jour
    fn mettre_a_jour(&mut self, mise_a_jour: MiseAJour) {
        let mut vols = self.vols.clone();
        if mise_a_jour.date != self.date {
            eprintln!("Mise a jour impossible: les dates ne sont pas les mêmes !");
        } else {
            for vol in &mut vols {
                if vol.numero_ogn == mise_a_jour.numero_ogn as i32 {
                    match mise_a_jour.champ_mis_a_jour.clone().as_str() {
                        "code_decollage" => {
                            vol.code_decollage = mise_a_jour.nouvelle_valeur.clone()
                        }
                        "machine_decollage" => {
                            vol.machine_decollage = mise_a_jour.nouvelle_valeur.clone()
                        }
                        "decolleur" => vol.decolleur = mise_a_jour.nouvelle_valeur.clone(),
                        "aeronef" => vol.aeronef = mise_a_jour.nouvelle_valeur.clone(),
                        "code_vol" => vol.code_vol = mise_a_jour.nouvelle_valeur.clone(),
                        "pilote1" => vol.pilote1 = mise_a_jour.nouvelle_valeur.clone(),
                        "pilote2" => vol.pilote2 = mise_a_jour.nouvelle_valeur.clone(),
                        "decollage" => {
                            vol.decollage = NaiveTime::parse_from_str(
                                format!("{}", mise_a_jour.nouvelle_valeur.clone()).as_str(),
                                "%H:%M",
                            )
                            .unwrap();
                        }
                        "atterissage" => {
                            vol.atterissage = NaiveTime::parse_from_str(
                                format!("{}", mise_a_jour.nouvelle_valeur.clone()).as_str(),
                                "%H:%M",
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
        self.vols = vols.clone();
    }
}

#[derive(Debug, PartialEq)]
pub struct MiseAJour {
    numero_ogn: u8,
    champ_mis_a_jour: String,
    nouvelle_valeur: String,
    pub date: NaiveDate,
}

impl MiseAJour {
    pub fn new() -> Self {
        MiseAJour {
            numero_ogn: u8::default(), //numero du vol **OGN**
            champ_mis_a_jour: String::default(),
            nouvelle_valeur: String::default(),
            date: NaiveDate::default(),
        }
    }

    pub fn parse(&mut self, texte_json: json::JsonValue) -> Result<(), String> {
        match texte_json {
            json::JsonValue::Object(objet) => {
                self.numero_ogn = objet["numero_ogn"].as_u8().unwrap_or_else(|| {
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

                self.date = NaiveDate::parse_from_str(
                    objet["date"].as_str().unwrap_or_else(|| {
                        eprintln!("pas la bonne valeur pour la nouvelle valeur");
                        ""
                    }),
                    "%Y/%m/%d",
                )
                .unwrap();
            }
            _ => {
                eprintln!("pas un objet json");
            }
        };
        Ok(())
    }
}

mod tests {

    #[test]
    fn mise_a_jour_parse_test() {
        use crate::planche::MiseAJour;
        use chrono::NaiveDate;
        use core::panic;

        let mise_a_jour_declaree = MiseAJour {
            numero_ogn: 1,
            champ_mis_a_jour: String::from("code_vol"),
            nouvelle_valeur: String::from("M"),
            date: NaiveDate::from_ymd_opt(2023, 04, 25).unwrap(),
        };

        let mut mise_a_jour_parse = MiseAJour::new();
        let _ = mise_a_jour_parse.parse(
            json::parse(
                "{ \
                    \"numero_ogn\": 1, \
                    \"champ_mis_a_jour\": \"code_vol\", \
                    \"nouvelle_valeur\": \"M\", \
                    \"date\":  \"2023/04/25\" \
                }",
            )
            .unwrap_or_else(|err| {
                panic!("{} : erreur !!", err);
            }),
        );

        assert_eq!(mise_a_jour_declaree, mise_a_jour_parse)
    }

    #[test]
    fn mettre_a_jour_test() {
        use crate::planche::{MettreAJour, MiseAJour, Planche};
        use crate::vol::Vol;
        use chrono::{NaiveDate, NaiveTime};

        let mut vols = Vec::new();
        vols.push(Vol {
            numero_ogn: 1,
            code_decollage: String::from("T"),
            machine_decollage: String::from("FREMA"),
            decolleur: String::from("YDL"),
            aeronef: String::from("F-CERJ"),
            code_vol: String::from("S"),
            pilote1: String::from("Walt Disney"),
            pilote2: String::default(),
            decollage: NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
            atterissage: NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
        });

        let date = NaiveDate::from_ymd_opt(2023, 04, 25).unwrap();

        let mise_a_jour = MiseAJour {
            numero_ogn: 1,
            champ_mis_a_jour: String::from("machine_decollage"),
            nouvelle_valeur: String::from("LUCIFER"),
            date: NaiveDate::from_ymd_opt(2023, 04, 25).unwrap(),
        };

        let mut planche = Planche { vols, date };
        planche.mettre_a_jour(mise_a_jour);

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
        let vols_verif = vec![vol_verif];
        let planche_verif = Planche {
            vols: vols_verif,
            date,
        };
        assert_eq!(planche, planche_verif)
    }
}
