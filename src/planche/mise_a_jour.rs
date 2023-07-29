use chrono::{NaiveDate, NaiveTime};

#[derive(Debug, PartialEq, Clone)]
pub struct MiseAJour {
    pub numero_ogn: i32,
    pub champ_mis_a_jour: String,
    pub nouvelle_valeur: String,
    pub date: NaiveDate,
    pub heure: NaiveTime,
}

impl Default for MiseAJour {
    fn default() -> Self {
        Self::new()
    }
}

impl MiseAJour {
    pub fn new() -> Self {
        MiseAJour {
            numero_ogn: i32::default(), //numero du vol **OGN**
            champ_mis_a_jour: String::default(),
            nouvelle_valeur: String::default(),
            date: NaiveDate::default(),
            heure: NaiveTime::default(),
        }
    }

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

pub trait MiseAJourJson {
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

pub trait MiseAJourObsoletes {
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
