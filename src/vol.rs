use crate::nom_fichier_date;
use chrono::{Datelike, NaiveDate, NaiveTime};
use json::JsonValue;
use std::fs;

#[derive(Clone, PartialEq, Debug)]
pub struct Vol {
    pub numero_ogn: i32,
    pub code_decollage: String,
    pub machine_decollage: String,
    pub decolleur: String,
    pub aeronef: String,
    pub code_vol: String,
    pub pilote1: String,
    pub pilote2: String,
    pub decollage: NaiveTime,
    pub atterissage: NaiveTime,
}

impl Default for Vol {
    fn default() -> Self {
        Vol {
            numero_ogn: 1,
            code_decollage: String::from("T"),
            machine_decollage: String::from("F-REMA"),
            decolleur: String::from("YDL"),
            aeronef: String::from("F-CERJ"),
            code_vol: String::from("S"),
            pilote1: String::from("Walt Disney"),
            pilote2: String::default(),
            decollage: NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
            atterissage: NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
        }
    }
}

impl Vol {
    fn _new() -> Self {
        Vol {
            numero_ogn: i32::default(),
            code_decollage: String::default(),
            machine_decollage: String::default(),
            decolleur: String::default(),
            aeronef: String::default(),
            code_vol: String::default(),
            pilote1: String::default(),
            pilote2: String::default(),
            decollage: NaiveTime::default(),
            atterissage: NaiveTime::default(),
        }
    }

    pub fn vers_json(&self) -> String {
        let vol = json::object! {
            numero_ogn: self.numero_ogn,
            code_decollage: *self.code_decollage,
            machine_decollage: *self.machine_decollage,
            decolleur: *self.decolleur,
            aeronef: *self.aeronef,
            code_vol: *self.code_vol,
            pilote1: *self.pilote1,
            pilote2: *self.pilote2,
            decollage: *self.decollage.format("%H:%M").to_string(),
            atterissage: *self.atterissage.format("%H:%M").to_string(),
        };
        vol.dump()
    }

    pub fn depuis_json(mut json_parse: JsonValue) -> Self {
        Vol {
            numero_ogn: json_parse["numero_ogn"].as_i32().unwrap_or_default(),
            code_decollage: json_parse["code_decollage"]
                .take_string()
                .unwrap_or_else(|| String::from("")),
            machine_decollage: json_parse["machine_decollage"]
                .take_string()
                .unwrap_or_else(|| String::from("")),
            decolleur: json_parse["decolleur"]
                .take_string()
                .unwrap_or_else(|| String::from("")),
            aeronef: json_parse["aeronef"]
                .take_string()
                .unwrap_or_else(|| String::from("")),
            code_vol: json_parse["code_vol"]
                .take_string()
                .unwrap_or_else(|| String::from("")),
            pilote1: json_parse["pilote1"]
                .take_string()
                .unwrap_or_else(|| String::from("")),
            pilote2: json_parse["pilote2"]
                .take_string()
                .unwrap_or_else(|| String::from("")),
            decollage: NaiveTime::parse_from_str(
                json_parse["decollage"].take_string().unwrap().as_str(),
                "%H:%M",
            )
            .unwrap(),
            atterissage: NaiveTime::parse_from_str(
                json_parse["atterissage"].take_string().unwrap().as_str(),
                "%H:%M",
            )
            .unwrap(),
        }
    }
}

pub trait VolJson {
    fn vers_json(self) -> String;
    fn depuis_json(&mut self, json: JsonValue);
    fn du(date: NaiveDate) -> Vec<Vol>;
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

    fn du(date: NaiveDate) -> Vec<Vol> {
        let annee = date.year();
        let mois = date.month();
        let jour = date.day();

        let mois_str = nom_fichier_date(mois as i32);
        let jour_str = nom_fichier_date(jour as i32);

        let fichiers = fs::read_dir(format!(
            "../site/dossier_de_travail/{}/{}/{}/",
            annee, mois_str, jour_str
        ))
        .unwrap();
        let mut vols: Vec<Vol> = Vec::new();

        for fichier in fichiers {
            let chemin_fichier = fichier.unwrap().file_name().into_string().unwrap();
            if chemin_fichier.clone() != *"affectations.json" {
                let vol_json = fs::read_to_string(format!(
                    "../site/dossier_de_travail/{}/{}/{}/{}",
                    annee,
                    mois_str,
                    jour_str,
                    chemin_fichier.clone()
                ))
                .unwrap_or_else(|err| {
                    log::error!(
                        "Impossible d'ouvrir le fichier {} : {}",
                        chemin_fichier.clone(),
                        err
                    );
                    String::from("")
                });
                let vol = Vol::depuis_json(json::parse(vol_json.as_str()).unwrap());
                vols.push(vol);
            }
        }
        vols
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
