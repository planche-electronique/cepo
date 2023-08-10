//! Gestion des mise à jour: envoi des modifications par champ de vol pour éviter de recharger
//! toute la planche à chaque fois

use chrono::{NaiveDate, NaiveTime};

/// Représentation en mémoire d'une "planche".
#[derive(Debug, PartialEq, Clone)]
pub struct MiseAJour {
    /// Le numero du vol sur OGN.
    pub numero_ogn: i32,
    /// Le nom du champ qui a été changé.
    pub champ_mis_a_jour: String,
    /// La nouvelle valeur de ce champ.
    pub nouvelle_valeur: String,
    /// La date du vol sur lequel le changement est fait.
    pub date: NaiveDate,
    /// L'heure à laquelle la requete est faite.
    pub heure: NaiveTime,
}

impl Default for MiseAJour {
    fn default() -> Self {
        Self::new()
    }
}

impl MiseAJour {
    /// Nouvelle mise à jour.
    pub fn new() -> Self {
        MiseAJour {
            numero_ogn: i32::default(), //numero du vol **OGN**
            champ_mis_a_jour: String::default(),
            nouvelle_valeur: String::default(),
            date: NaiveDate::default(),
            heure: NaiveTime::default(),
        }
    }
    /// Pour parser une mise à jour depuis un texte json, préalablement parsé à l'aide de [`json::parse()`].
    pub fn parse(&mut self, texte_json: json::JsonValue) -> Result<(), String> {
        match texte_json {
            json::JsonValue::Object(objet) => {
                self.numero_ogn = objet["numero_ogn"].as_i32().unwrap_or_else(|| {
                    log::error!("pas de numero de vol dans la requete");
                    0
                });

                self.champ_mis_a_jour = objet["champ_mis_a_jour"]
                    .as_str()
                    .unwrap_or_else(|| {
                        log::error!("pas le bon champ pour la nouvelle valeur");
                        ""
                    })
                    .to_string();

                self.nouvelle_valeur = objet["nouvelle_valeur"]
                    .as_str()
                    .unwrap_or_else(|| {
                        log::error!("pas la bonne valeur pour la nouvelle valeur");
                        ""
                    })
                    .to_string();

                self.date = NaiveDate::parse_from_str(
                    objet["date"].as_str().unwrap_or_else(|| {
                        log::error!("pas la bonne valeur pour la nouvelle valeur");
                        ""
                    }),
                    "%Y/%m/%d",
                )
                .unwrap();

                self.heure = chrono::Local::now().time();
            }
            _ => {
                eprintln!("pas un objet json");
            }
        };
        Ok(())
    }

    /// Pour encoder en Json.
    pub fn vers_json(&self) -> String {
        json::object! {
            numero_ogn: self.numero_ogn,
            date: *self.date.format("%Y/%m/%d").to_string(),
            champ_mis_a_jour: *self.champ_mis_a_jour,
            nouvelle_valeur: *self.nouvelle_valeur,
            heure: *self.heure.format("%H:%M").to_string(),
        }
        .dump()
    }
}

/// S'occupe des relations entre plusieurs mises à jour et Json.
pub trait MiseAJourJson {
    /// Encode plusieurs mises à jour en Json.
    fn vers_json(&self) -> String;
}

impl MiseAJourJson for Vec<MiseAJour> {
    fn vers_json(&self) -> String {
        let mut string = String::new();
        string.push('[');
        for maj in self {
            string.push_str(maj.vers_json().as_str());
            string.push(',')
        }
        if string != *"[" {
            string.pop();
        }
        string.push(']');
        string
    }
}

/// S'occupe des mises a jour trop vieilles.
pub trait MiseAJourObsoletes {
    /// Pour supprimer les mises a jour de plus d'un certain temps.
    fn enlever_majs_obsoletes(&mut self, temps: chrono::Duration);
}

impl MiseAJourObsoletes for Vec<MiseAJour> {
    fn enlever_majs_obsoletes(&mut self, temps: chrono::Duration) {
        let heure_actuelle = chrono::Local::now().time();
        let mut i = 0;
        while i < self.len() {
            if (heure_actuelle - self[i].heure) > temps {
                self.remove(i);
            } else {
                i += 1;
            }
        }
    }
}
