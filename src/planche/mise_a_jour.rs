use chrono::NaiveDate;

#[derive(Debug, PartialEq, Clone)]
pub struct MiseAJour {
    pub numero_ogn: u8,
    pub champ_mis_a_jour: String,
    pub nouvelle_valeur: String,
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
            }
            _ => {
                eprintln!("pas un objet json");
            }
        };
        Ok(())
    }

    pub fn vers_json(self: &Self) -> String {
        json::object! {
            numero_ogn: self.numero_ogn,
            date: *self.date.format("%Y/%m/%d").to_string(),
            champ_mis_a_jour: *self.champ_mis_a_jour,
            nouvelle_valeur: *self.nouvelle_valeur
        }
        .dump()
    }
}

pub trait MiseAJourJson {
    fn vers_json(self: &Self) -> String;
}

impl MiseAJourJson for Vec<MiseAJour> {
    fn vers_json(self: &Self) -> String {
        let mut string = String::new();
        string.push_str("[");
        for maj in self {
            string.push_str(maj.vers_json().as_str());
            string.push_str(",")
        }
        string.push_str("]");
        return string;
    }
}
