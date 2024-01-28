//! Tout ce qui attrait aux vols que nous enregistrons.

use crate::{creer_chemin_jour, data_dir, nom_fichier_date, Context};
use async_trait::async_trait;
use brick_ogn::flight::Flight;
use chrono::{Datelike, NaiveDate, NaiveTime};
use json::JsonValue;
use std::fs;
use serde_json;

/*
/// Un trait qui permet d'encoder/décoder des vols en JSON.
pub trait FlightJson {
    /// Permet d'encoder un vol en JSON.
    fn to_json(self) -> String;
    /// Décode un vol depuis un JsonValue, qui peut être lui-même parsé en utilisant
    /// json::parse!(string).
    fn from_json(&mut self, json: JsonValue);
}

impl FlightJson for Vec<Flight> {
    fn to_json(self) -> String {
        //on crée une string qui sera la json final et on lui rajoute le dbut d'un tableau
        let mut vols_str = String::new();
        vols_str.push_str("[\n");

        //pour chaque vol on ajoute sa version json a vols_str et on rajoute une virgule
        for vol in self {
            vols_str.push_str(vol.vers_json().as_str());
            vols_str.push(',');
        }
        vols_str = vols_str[0..(vols_str.len() - 1)].to_string(); // on enleve la virgule de trop
        vols_str.push_str("\n]");
        vols_str
    }

    fn depuis_json(&mut self, json: JsonValue) {
        let mut vols = Vec::new();
        for vol in json.members() {
            vols.push(Flight::depuis_json(vol.clone()));
        }
        (*self) = vols;
    }
}
*/

/// Interactions enter le disque et des vols, généralement sous la forme d'un Vec\<Flight\>.
#[async_trait]
pub trait FlightSaving {
    /// Enregistrer des vols sur le disque à partir d'une date à l'adresse `$XDG_DATA_DIR/cepo/annee/mois/jour`.
    fn save(&self, date: NaiveDate);
    /// Charger des vols sur le disque à partir d'une date à l'adresse `$XDG_DATA_DIR/cepo/annee/mois/jour`.
    fn load(date: NaiveDate)
        -> Result<Vec<Flight>, Box<dyn std::error::Error + Send + Sync>>;
    /// Charge les vols depuis le disque et les mets égalemen à jour par une requête au serveur OGN.
    async fn from_day(
        date: NaiveDate,
        context: &Context,
    ) -> Result<Vec<Flight>, Box<dyn std::error::Error + Send + Sync>>;
}

#[async_trait]
impl FlightSaving for Vec<Flight> {
    fn save(&self, date: NaiveDate) {
        let vols = self.clone();
        let annee = date.year();
        let mois = date.month();
        let jour = date.day();

        let jour_str = nom_fichier_date(jour as i32);
        let mois_str = nom_fichier_date(mois as i32);

        log::info!(
            "Enregistrement des vols du {}/{}/{}",
            annee,
            mois_str,
            jour_str
        );

        creer_chemin_jour(annee, mois, jour);

        for (index, vol) in vols.iter().enumerate() {
            let index_str = nom_fichier_date(index as i32);
            let flight_string = serde_json::to_string(vol).unwrap_or_default();
            let mut vols_path = crate::data_dir();
            vols_path.push(format!("{annee}/{mois_str}/{jour_str}/{index_str}.json"));
            let mut fichier = String::new();
            if vols_path.exists() {
                fichier = fs::read_to_string(&vols_path).unwrap_or_else(|err| {
                    log::error!(
                        "fichier numero {} de chemin {:?} introuvable ou non ouvrable : {}",
                        index,
                        &vols_path,
                        err.to_string()
                    );
                    "".to_string()
                });
            }
            
            if fichier != flight_string {
                fs::write(&vols_path, flight_string).unwrap_or_else(|err| {
                    log::error!(
                        "Impossible d'écrire le fichier du jour {}/{}/{} et d'index {} : {}",
                        annee,
                        mois_str,
                        jour_str,
                        index,
                        err
                    );
                });
            }
        }
    }

    fn load(
        date: NaiveDate,
    ) -> Result<Vec<Flight>, Box<dyn std::error::Error + Send + Sync>> {
        let annee = date.year();
        let mois = date.month();
        let jour = date.day();

        let mois_str = nom_fichier_date(mois as i32);
        let jour_str = nom_fichier_date(jour as i32);

        log::info!("Lecture des fichiers de vol du {annee}/{mois_str}/{jour_str}");

        creer_chemin_jour(annee, mois, jour);
        let mut chemin = data_dir();
        chemin.push(format!("{}/{}/{}/", annee, mois_str, jour_str));
        let fichiers = fs::read_dir(&chemin).unwrap_or_else(|_| panic!("Couldn't load {:?}", chemin.clone()));
        let mut vols: Vec<Flight> = Vec::new();

        for fichier in fichiers {
            let file_name = fichier.unwrap().file_name().into_string().unwrap();
            let file_path = chemin.as_path().join(std::path::Path::new(&file_name));
            if &file_name != "affectations.json" {
                let vol_json = fs::read_to_string(file_path).unwrap_or_else(|err| {
                    log::error!("Impossible d'ouvrir le fichier {} : {}", file_name, err);
                    String::from("")
                });
                let vol = serde_json::from_str(&vol_json)?;
                vols.push(vol);
            }
        }
        Ok(vols)
    }

    async fn from_day(
        date: NaiveDate,
        context: &Context,
    ) -> Result<Vec<Flight>, Box<dyn std::error::Error + Send + Sync>> {
        let vols = Vec::load(date).unwrap();
        // looks to be unuseful no ?
        //there should be a force trigger but it is normally complete as it is not today's flights
        //vols.mettre_a_jour(vols_ogn(date, actif_serveur.configuration.oaci.clone()).await?);
        vols.save(date);
        Ok(vols)
    }
}

/// Un trait pour ajouter les nouvelles valeurs d'heures à un vol sans changer ses champs rentrés et fixes (ex: pilote).
pub trait Update {
    /// ajouter les nouvelles valeurs d'heures à un vol sans changer ses champs rentrés et fixes (ex: pilote).
    fn update(&mut self, nouveaux_vols: Vec<Flight>);
}

impl Update for Vec<Flight> {
    fn update(&mut self, derniers_vols: Vec<Flight>) {
        //on teste les égalités et on remplace si besoin
        let mut rang_prochain_vol = 0;
        let mut priorite_prochain_vol = 0;
        #[allow(unused_assignments)]
        for (mut rang_nouveau_vol, nouveau_vol) in derniers_vols.into_iter().enumerate() {
            let mut existe = false;
            for ancien_vol in &mut *self {
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
                self[rang_prochain_vol].ogn_nb = nouveau_vol.ogn_nb;
                self[rang_prochain_vol].takeoff_code = nouveau_vol.takeoff_code.clone();
                self[rang_prochain_vol].takeoff = nouveau_vol.takeoff;
                self[rang_prochain_vol].landing = nouveau_vol.landing;
            }
            if !existe {
                self.push(nouveau_vol);
            }
            rang_nouveau_vol += 1;
        }
    }
}
