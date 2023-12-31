//! Module des planche, i.e. un ensemble de plusieurs [`Vol`] et d'affectation.

pub mod mise_a_jour;

use crate::ogn::vols_ogn;
use crate::vol::{ChargementVols, Vol, VolJson};
use crate::{creer_chemin_jour, nom_fichier_date, ActifServeur};
use chrono::{Datelike, NaiveDate, NaiveTime};
use json;
use log;
pub use mise_a_jour::MiseAJour;
use std::fs;

/// Représentation des données de vol d'une journée, en cours.
#[derive(PartialEq, Debug, Clone)]
pub struct Planche {
    /// Tous les vols d'un jour.
    pub vols: Vec<Vol>,
    /// La date de ce jour.
    pub date: NaiveDate,
    /// le pilote de treuil.
    pub pilote_tr: String, // parmi pilotes_tr
    /// Le treuil en service.
    pub treuil: String, // parmi treuils
    /// Le pilote de remorqueur en service.
    pub pilote_rq: String, // parmi pilotes_rq
    /// Le remorqueur en service.
    pub remorqueur: String, // parmi remorqueurs
    /// Le chef de piste en service.
    pub chef_piste: String, // parmi pilotes
}

impl Default for Planche {
    fn default() -> Self {
        Self::new()
    }
}

impl Planche {
    /// Vols chargés depuis le disque et mis à jour depuis OGN.
    pub async fn du(
        date: NaiveDate,
        actif_serveur: &ActifServeur,
    ) -> Result<Planche, Box<dyn std::error::Error + Send + Sync>> {
        let annee = date.year();
        let mois = date.month();
        let jour = date.day();

        creer_chemin_jour(annee, mois, jour);
        let mut planche = Planche::depuis_disque(date).unwrap();
        //planche.mettre_a_jour_ogn(actif_serveur).await?;
        planche.enregistrer();
        Ok(planche)
    }

    /// Mise à jour de la planche à l'aide d'une requête OGN.
    pub async fn mettre_a_jour_ogn(
        &mut self,
        actif_serveur: &ActifServeur,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let date = chrono::Local::now().date_naive();
        //on teste les égalités et on remplace si besoin
        let derniers_vols = vols_ogn(date, actif_serveur.configuration.oaci.clone()).await?;
        let mut rang_prochain_vol = 0;
        let mut priorite_prochain_vol = 0;
        let ancienne_planche = self;
        #[allow(unused_assignments)]
        for (mut rang_nouveau_vol, nouveau_vol) in derniers_vols.into_iter().enumerate() {
            let mut existe = false;
            for ancien_vol in &mut ancienne_planche.vols {
                // si on est sur le meme vol
                if nouveau_vol.numero_ogn == ancien_vol.numero_ogn {
                    existe = true;
                    let heure_default = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
                    //teste les différentes valeurs qui peuvent être mises a jour
                    if ancien_vol.decollage == heure_default {
                        ancien_vol.decollage = nouveau_vol.decollage;
                    }
                    if ancien_vol.atterissage == heure_default {
                        ancien_vol.atterissage = nouveau_vol.atterissage;
                    }
                } else if nouveau_vol.aeronef == ancien_vol.aeronef {
                    if priorite_prochain_vol != 0 {
                        if priorite_prochain_vol < nouveau_vol.numero_ogn
                            && nouveau_vol.numero_ogn < 0
                        {
                            existe = true;
                            priorite_prochain_vol = nouveau_vol.numero_ogn;
                            rang_prochain_vol = rang_nouveau_vol;
                        }
                    } else if nouveau_vol.numero_ogn < 0 && priorite_prochain_vol == 0 {
                        existe = true;
                        priorite_prochain_vol = nouveau_vol.numero_ogn;
                        rang_prochain_vol = rang_nouveau_vol;
                    }
                }
            }
            if priorite_prochain_vol != 0 {
                // on recupere le vol affecté avec le plus de priorité et on lui affecte les données de ogn
                ancienne_planche.vols[rang_prochain_vol].numero_ogn = nouveau_vol.numero_ogn;
                ancienne_planche.vols[rang_prochain_vol].code_decollage =
                    nouveau_vol.code_decollage.clone();
                ancienne_planche.vols[rang_prochain_vol].decollage = nouveau_vol.decollage;
                ancienne_planche.vols[rang_prochain_vol].atterissage = nouveau_vol.atterissage;
            }
            if !existe {
                ancienne_planche.vols.push(nouveau_vol);
            }
            rang_nouveau_vol += 1;
        }
        Ok(())
    }

    /// Chargement de la planche depuis le disque.
    pub fn depuis_disque(
        date: NaiveDate,
    ) -> Result<Planche, Box<dyn std::error::Error + Send + Sync>> {
        let annee = date.year();
        let mois = date.month();
        let jour = date.day();
        log::info!(
            "Chargement depuis le disque de la planche du {}/{}/{}",
            annee,
            mois,
            jour
        );

        let mois_str = nom_fichier_date(mois as i32);
        let jour_str = nom_fichier_date(jour as i32);

        let vols: Vec<Vol> = Vec::depuis_disque(date).unwrap();
        let mut affectations_path = crate::data_dir();
        affectations_path.push(format!(
            "{}/{}/{}/affectations.json",
            annee, mois_str, jour_str
        ));
        let affectations_str = fs::read_to_string(affectations_path).unwrap_or_default();
        let affectations_json =
            json::parse(&affectations_str).unwrap_or_else(|_| json::JsonValue::Null);
        let pilote_tr = affectations_json["pilote_tr"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let treuil = affectations_json["treuil"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let pilote_rq = affectations_json["pilote_rq"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let remorqueur = affectations_json["remorqueur"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let chef_piste = affectations_json["chef_piste"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        Ok(Planche {
            date,
            vols,
            pilote_tr,
            treuil,
            pilote_rq,
            remorqueur,
            chef_piste,
        })
    }
    /// Enregistrement de la planche sur le disque
    pub fn enregistrer(&self) {
        let date = self.date;
        let annee = date.year();
        let mois = date.month();
        let jour = date.day();

        let jour_str = nom_fichier_date(jour as i32);
        let mois_str = nom_fichier_date(mois as i32);

        self.vols.enregistrer(date);

        let mut affectations_path = crate::data_dir();
        affectations_path.push(format!(
            "{}/{}/{}/affectations.json",
            annee, mois_str, jour_str
        ));
        let affectations_fichier = fs::read_to_string(&affectations_path).unwrap_or_default();
        let affectations = json::object! {
            "pilote_tr": self.pilote_tr.clone(),
            "treuil": self.treuil.clone(),
            "pilote_rq": self.pilote_rq.clone(),
            "remorqueur": self.remorqueur.clone(),
            "chef_piste": self.chef_piste.clone(),
        };
        if json::stringify(affectations.clone()) != affectations_fichier {
            fs::write(&affectations_path, json::stringify(affectations.clone())).unwrap_or_else(
                |err| {
                    log::error!("Impossible d'écrire les affectations : {}", err);
                },
            )
        }
        log::info!("Affectations du {annee}/{mois_str}/{jour_str} enregistrees.");
    }
    /// Une nouvelle planche.
    pub fn new() -> Self {
        Planche {
            vols: Vec::new(),
            date: NaiveDate::default(),
            pilote_tr: String::new(),
            treuil: String::new(),
            pilote_rq: String::new(),
            remorqueur: String::new(),
            chef_piste: String::new(),
        }
    }

    /// Encodage de la planche en Json.
    pub fn vers_json(self) -> String {
        let vols_json = self.vols.vers_json();
        let date_json = self.date.format("%Y/%m/%d").to_string();
        let reste_json = json::stringify(json::object! {
            pilote_tr: self.pilote_tr,
            treuil: self.treuil,
            pilote_rq: self.pilote_rq,
            remorqueur: self.remorqueur,
            chef_piste: self.chef_piste,
        });
        let mut json = String::new();
        json.push_str("{ \"date\": \"");
        json.push_str(&date_json);
        json.push_str("\",\n\"vols\" : ");
        json.push_str(&vols_json);
        json.push_str(", \n \"affectations\": ");
        json.push_str(&reste_json);
        json.push('\n');
        json.push('}');
        json
    }
}

/// Mise à jour d'une planche à l'aide d'une [`MiseAJour`].
pub trait MettreAJour {
    /// Mise à jour d'une planche à l'aide d'une [`MiseAJour`].
    fn mettre_a_jour(&mut self, mise_a_jour: MiseAJour);
}

impl MettreAJour for Planche {
    // on crée une fonction pour mettre la mise à jour dans le vecteur Vols du jour
    fn mettre_a_jour(&mut self, mise_a_jour: MiseAJour) {
        let mut vols = self.vols.clone();
        if mise_a_jour.date != self.date {
            log::error!("Mise a jour impossible: les dates ne sont pas les mêmes !");
        } else if mise_a_jour.champ_mis_a_jour.clone() == "nouveau" {
            vols.push(Vol {
                numero_ogn: mise_a_jour.numero_ogn,
                aeronef: mise_a_jour.nouvelle_valeur.clone(),
                code_vol: String::new(),
                code_decollage: String::new(),
                machine_decollage: String::new(),
                decolleur: String::new(),
                pilote1: String::new(),
                pilote2: String::new(),
                decollage: NaiveTime::default(),
                atterissage: NaiveTime::default(),
            });
        } else if mise_a_jour.champ_mis_a_jour.clone() == "supprimer" {
            vols.retain(|vol| vol.numero_ogn != mise_a_jour.numero_ogn);
        } else {
            for vol in &mut vols {
                if vol.numero_ogn == mise_a_jour.numero_ogn {
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
                                &mise_a_jour.nouvelle_valeur.clone(),
                                "%H:%M",
                            )
                            .unwrap();
                        }
                        "atterissage" => {
                            vol.atterissage = NaiveTime::parse_from_str(
                                &mise_a_jour.nouvelle_valeur.clone(),
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
            if mise_a_jour.numero_ogn == 0 {
                match mise_a_jour.champ_mis_a_jour.as_str() {
                    "pilote_tr" => self.pilote_tr = mise_a_jour.nouvelle_valeur,
                    "treuil" => self.treuil = mise_a_jour.nouvelle_valeur,
                    "pilote_rq" => self.pilote_rq = mise_a_jour.nouvelle_valeur,
                    "remorqueur" => self.remorqueur = mise_a_jour.nouvelle_valeur,
                    "chef_piste" => self.chef_piste = mise_a_jour.nouvelle_valeur,
                    _ => log::warn!(
                        "la mise a jour pour le {} à {} ne contient pas le bon champ",
                        mise_a_jour.date.format("%Y/%m/%d"),
                        mise_a_jour.heure.format("%H:%M")
                    ),
                }
            }
        }
        self.vols = vols.clone();
    }
}

mod tests {

    #[test]
    fn mise_a_jour_parse_test() {
        use crate::planche::MiseAJour;
        use chrono::{NaiveDate, NaiveTime};
        use core::panic;

        let mise_a_jour_declaree = MiseAJour {
            numero_ogn: 1,
            champ_mis_a_jour: String::from("code_vol"),
            nouvelle_valeur: String::from("M"),
            date: NaiveDate::from_ymd_opt(2023, 04, 25).unwrap(),
            heure: NaiveTime::default(),
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
        mise_a_jour_parse.heure = NaiveTime::default();

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
            heure: NaiveTime::default(),
        };

        let mut planche = Planche {
            vols,
            date,
            pilote_tr: String::new(),
            treuil: String::new(),
            pilote_rq: String::new(),
            remorqueur: String::new(),
            chef_piste: String::new(),
        };
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
            pilote_tr: String::new(),
            treuil: String::new(),
            pilote_rq: String::new(),
            remorqueur: String::new(),
            chef_piste: String::new(),
        };
        assert_eq!(planche, planche_verif)
    }
}
