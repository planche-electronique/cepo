//! Tout ce qui attrait aux vols que nous enregistrons.

use crate::{creer_chemin_jour, data_dir, nom_fichier_date, ActifServeur};
use async_trait::async_trait;
use brick_ogn::vol::Vol;
use chrono::{Datelike, NaiveDate, NaiveTime};
use json::JsonValue;
use std::fs;

/// Un trait qui permet d'encoder/décoder des vols en JSON.
pub trait VolJson {
    /// Permet d'encoder un vol en JSON.
    fn vers_json(self) -> String;
    /// Décode un vol depuis un JsonValue, qui peut être lui-même parsé en utilisant
    /// json::parse!(string).
    fn depuis_json(&mut self, json: JsonValue);
}

impl VolJson for Vec<Vol> {
    fn vers_json(self) -> String {
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
            vols.push(Vol::depuis_json(vol.clone()));
        }
        (*self) = vols;
    }
}

/// Interactions enter le disque et des vols, généralement sous la forme d'un Vec\<Vol\>.
#[async_trait]
pub trait ChargementVols {
    /// Enregistrer des vols sur le disque à partir d'une date à l'adresse `$XDG_DATA_DIR/cepo/annee/mois/jour`.
    fn enregistrer(&self, date: NaiveDate);
    /// Charger des vols sur le disque à partir d'une date à l'adresse `$XDG_DATA_DIR/cepo/annee/mois/jour`.
    fn depuis_disque(date: NaiveDate)
        -> Result<Vec<Vol>, Box<dyn std::error::Error + Send + Sync>>;
    /// Charge les vols depuis le disque et les mets égalemen à jour par une requête au serveur OGN.
    async fn du(
        date: NaiveDate,
        actif_serveur: &ActifServeur,
    ) -> Result<Vec<Vol>, Box<dyn std::error::Error + Send + Sync>>;
}

#[async_trait]
impl ChargementVols for Vec<Vol> {
    fn enregistrer(&self, date: NaiveDate) {
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

            if fichier != vol.vers_json() {
                fs::write(&vols_path, vol.vers_json()).unwrap_or_else(|err| {
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

    fn depuis_disque(
        date: NaiveDate,
    ) -> Result<Vec<Vol>, Box<dyn std::error::Error + Send + Sync>> {
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
        let mut vols: Vec<Vol> = Vec::new();

        for fichier in fichiers {
            let file_name = fichier.unwrap().file_name().into_string().unwrap();
            let file_path = chemin.as_path().join(std::path::Path::new(&file_name));
            if &file_name != "affectations.json" {
                let vol_json = fs::read_to_string(file_path).unwrap_or_else(|err| {
                    log::error!("Impossible d'ouvrir le fichier {} : {}", file_name, err);
                    String::from("")
                });
                let vol = Vol::depuis_json(json::parse(vol_json.as_str()).unwrap());
                vols.push(vol);
            }
        }
        Ok(vols)
    }

    async fn du(
        date: NaiveDate,
        _actif_serveur: &ActifServeur,
    ) -> Result<Vec<Vol>, Box<dyn std::error::Error + Send + Sync>> {
        let vols = Vec::depuis_disque(date).unwrap();
        // looks to be unuseful no ?
        //there should be a force trigger but it is normally complete as it is not today's flights
        //vols.mettre_a_jour(vols_ogn(date, actif_serveur.configuration.oaci.clone()).await?);
        vols.enregistrer(date);
        Ok(vols)
    }
}

/// Un trait pour ajouter les nouvelles valeurs d'heures à un vol sans changer ses champs rentrés et fixes (ex: pilote).
pub trait MettreAJour {
    /// ajouter les nouvelles valeurs d'heures à un vol sans changer ses champs rentrés et fixes (ex: pilote).
    fn mettre_a_jour(&mut self, nouveaux_vols: Vec<Vol>);
}

impl MettreAJour for Vec<Vol> {
    fn mettre_a_jour(&mut self, derniers_vols: Vec<Vol>) {
        //on teste les égalités et on remplace si besoin
        let mut rang_prochain_vol = 0;
        let mut priorite_prochain_vol = 0;
        #[allow(unused_assignments)]
        for (mut rang_nouveau_vol, nouveau_vol) in derniers_vols.into_iter().enumerate() {
            let mut existe = false;
            for ancien_vol in &mut *self {
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
                self[rang_prochain_vol].numero_ogn = nouveau_vol.numero_ogn;
                self[rang_prochain_vol].code_decollage = nouveau_vol.code_decollage.clone();
                self[rang_prochain_vol].decollage = nouveau_vol.decollage;
                self[rang_prochain_vol].atterissage = nouveau_vol.atterissage;
            }
            if !existe {
                self.push(nouveau_vol);
            }
            rang_nouveau_vol += 1;
        }
    }
}

mod tests {

    #[test]
    fn vec_vol_vers_json_test() {
        use crate::vol::{Vol, VolJson};

        let vols = vec![Vol::default()];
        let vols_str = vols.vers_json();

        assert_eq!(vols_str, String::from("[\n{\"numero_ogn\":1,\"code_decollage\":\"T\",\"machine_decollage\":\"F-REMA\",\"decolleur\":\"YDL\",\"aeronef\":\"F-CERJ\",\"code_vol\":\"S\",\"pilote1\":\"Walt Disney\",\"pilote2\":\"\",\"decollage\":\"13:00\",\"atterissage\":\"14:00\"}\n]"))
    }
}
