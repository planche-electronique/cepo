//! Module des planche, i.e. un ensemble de plusieurs [`Vol`] et d'affectation.

pub mod mise_a_jour;

use crate::ogn::vols_ogn;
use crate::vol::ChargementVols;
use crate::{creer_chemin_jour, nom_fichier_date, ActifServeur};
use async_trait::async_trait;
use brick_ogn::planche::Planche;
use brick_ogn::vol::Vol;
use chrono::{Datelike, NaiveDate, NaiveTime};
use json;
use log;
pub use mise_a_jour::MiseAJour;
use std::fs;

/// Un trait qui a pour attrait de s'occuper du stockage (chargement depuyis
/// le disque et vers le disque du type planche mais aussi plus general).
#[async_trait]
pub trait Stockage {
    /// Vols chargés depuis le disque et mis à jour depuis OGN.
    async fn du(
        date: NaiveDate,
        actif_serveur: &ActifServeur,
    ) -> Result<Planche, Box<dyn std::error::Error + Send + Sync>>;
    /// Mise à jour de la planche à l'aide d'une requête OGN.
    async fn mettre_a_jour_ogn(
        &mut self,
        actif_serveur: &ActifServeur,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    /// Chargement de la planche depuis le disque.
    fn depuis_disque(date: NaiveDate) -> Result<Planche, Box<dyn std::error::Error + Send + Sync>>;
    /// Enregistrement de la planche sur le disque
    async fn enregistrer(&self);
}

#[async_trait]
impl Stockage for Planche {
    async fn du(
        date: NaiveDate,
        actif_serveur: &ActifServeur,
    ) -> Result<Planche, Box<dyn std::error::Error + Send + Sync>> {
        let annee = date.year();
        let mois = date.month();
        let jour = date.day();

        creer_chemin_jour(annee, mois, jour);
        let mut planche = Planche::depuis_disque(date).unwrap();
        planche.mettre_a_jour_ogn(actif_serveur).await?;
        let _ = planche.enregistrer();
        Ok(planche)
    }

    async fn mettre_a_jour_ogn(
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

    fn depuis_disque(date: NaiveDate) -> Result<Planche, Box<dyn std::error::Error + Send + Sync>> {
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
    async fn enregistrer(&self) {
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
}
