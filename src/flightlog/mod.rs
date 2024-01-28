//! Module des planche, i.e. un ensemble de plusieurs [`Vol`] et d'affectation.

use crate::ogn::ogn_flights;
use crate::{create_fs_path_day, nb_2digits_string, Context};
use async_trait::async_trait;
use brick_ogn::flightlog::FlightLog;
use chrono::{Datelike, NaiveDate, NaiveTime};
use log;
pub use brick_ogn::flightlog::update::Update;
use std::fs;
/// Un trait qui a pour attrait de s'occuper du stockage (chargement depuyis
/// le disque et vers le disque du type planche mais aussi plus general).
#[async_trait]
pub trait Storage {
    /// Vols chargés depuis le disque et mis à jour depuis OGN.
    async fn from_day(
        date: NaiveDate,
        actif_serveur: &Context,
    ) -> Result<FlightLog, Box<dyn std::error::Error + Send + Sync>>;
    /// Mise à jour de la planche à l'aide d'une requête OGN.
    async fn update_ogn(
        &mut self,
        actif_serveur: &Context,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    /// Chargement de la planche depuis le disque.
    fn load(date: NaiveDate) -> Result<FlightLog, Box<dyn std::error::Error + Send + Sync>>;
    /// Enregistrement de la planche sur le disque
    fn save(&self);
}

#[async_trait]
impl Storage for FlightLog {
    async fn from_day(
        date: NaiveDate,
        actif_serveur: &Context,
    ) -> Result<FlightLog, Box<dyn std::error::Error + Send + Sync>> {
        let year: i32 = date.year();
        let month = date.month();
        let day = date.day();

        create_fs_path_day(year, month, day);
        let mut planche = FlightLog::load(date).unwrap();
        planche.update_ogn(actif_serveur).await?;
        let _ = planche.save();
        Ok(planche)
    }

    async fn update_ogn(
        &mut self,
        actif_serveur: &Context,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let date = chrono::Local::now().date_naive();
        //on teste les égalités et on remplace si besoin
        let last_flights = ogn_flights(date, actif_serveur.configuration.oaci.clone()).await?;
        let mut rang_prochain_vol = 0;
        let mut priorite_prochain_vol = 0;
        let ancienne_planche = self;
        #[allow(unused_assignments)]
        for (mut rang_nouveau_vol, nouveau_vol) in last_flights.into_iter().enumerate() {
            let mut existe = false;
            for ancien_vol in &mut ancienne_planche.flights {
                // si on est sur le meme vol
                if nouveau_vol.ogn_nb == ancien_vol.ogn_nb {
                    existe = true;
                    let heure_default = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
                    //teste les différentes valeurs qui peuvent être mises a jour
                    if ancien_vol.takeoff == heure_default {
                        ancien_vol.takeoff = nouveau_vol.takeoff;
                    }
                    if ancien_vol.landing == heure_default {
                        ancien_vol.landing = nouveau_vol.landing;
                    }
                } else if nouveau_vol.glider == ancien_vol.glider {
                    if priorite_prochain_vol != 0 {
                        if priorite_prochain_vol < nouveau_vol.ogn_nb
                            && nouveau_vol.ogn_nb < 0
                        {
                            existe = true;
                            priorite_prochain_vol = nouveau_vol.ogn_nb;
                            rang_prochain_vol = rang_nouveau_vol;
                        }
                    } else if nouveau_vol.ogn_nb < 0 && priorite_prochain_vol == 0 {
                        existe = true;
                        priorite_prochain_vol = nouveau_vol.ogn_nb;
                        rang_prochain_vol = rang_nouveau_vol;
                    }
                }
            }
            if priorite_prochain_vol != 0 {
                // on recupere le vol affecté avec le plus de priorité et on lui affecte les données de ogn
                ancienne_planche.flights[rang_prochain_vol].ogn_nb = nouveau_vol.ogn_nb;
                ancienne_planche.flights[rang_prochain_vol].takeoff_code =
                    nouveau_vol.takeoff_code.clone();
                ancienne_planche.flights[rang_prochain_vol].takeoff = nouveau_vol.takeoff;
                ancienne_planche.flights[rang_prochain_vol].landing = nouveau_vol.landing;
            }
            if !existe {
                ancienne_planche.flights.push(nouveau_vol);
            }
            rang_nouveau_vol += 1;
        }
        Ok(())
    }

    fn load(date: NaiveDate) -> Result<FlightLog, Box<dyn std::error::Error + Send + Sync>> {
        let annee = date.year();
        let mois = date.month();
        let jour = date.day();
        log::info!(
            "Loading FlightLog from the disk {}/{}/{}",
            annee,
            mois,
            jour
        );

        let mois_str = nb_2digits_string(mois as i32);
        let jour_str = nb_2digits_string(jour as i32);

        let mut path = crate::data_dir();
        path.push(format!(
            "{}/{}/{}/affectations.json",
            annee, mois_str, jour_str
        ));

        let flightlog_str = fs::read_to_string(path).unwrap_or_default();
        let flightlog = serde_json::from_str(&flightlog_str)?;

        Ok(flightlog)
    }

    fn save(&self) {
        let date = self.date;
        let annee = date.year();
        let mois = date.month();
        let jour = date.day();

        let jour_str = nb_2digits_string(jour as i32);
        let mois_str = nb_2digits_string(mois as i32);

        let mut file_path = crate::data_dir();
        file_path.push(format!(
            "{}/{}/{}/affectations.json",
            annee, mois_str, jour_str
        ));

        fs::write(&file_path, serde_json::to_string(self).unwrap_or_default()).unwrap();

        log::info!("Affectations du {annee}/{mois_str}/{jour_str} enregistrees.");
    }
}
