use chrono::NaiveTime;
use json::JsonValue;


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
    
    fn _default() -> Self {
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
    
    pub fn to_json(self: &Self) -> String {
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

    pub fn from_json(mut json_parse: JsonValue) -> Self {
        Vol {
            numero_ogn: json_parse["numero_ogn"].as_i32().unwrap_or_default(),
            code_decollage: json_parse["code_decollage"].take_string().unwrap_or_else(||{String::from("")}),
            machine_decollage: json_parse["machine_decollage"].take_string().unwrap_or_else(||{String::from("")}),
            decolleur: json_parse["decolleur"].take_string().unwrap_or_else(||{String::from("")}),
            aeronef: json_parse["aeronef"].take_string().unwrap_or_else(||{String::from("")}),
            code_vol: json_parse["code_vol"].take_string().unwrap_or_else(||{String::from("")}),
            pilote1: json_parse["pilote1"].take_string().unwrap_or_else(||{String::from("")}),
            pilote2: json_parse["pilote2"].take_string().unwrap_or_else(||{String::from("")}),
            decollage: NaiveTime::parse_from_str(json_parse["decollage"].take_string().unwrap().as_str(), "%Hh%M").unwrap(),
            atterissage: NaiveTime::parse_from_str(json_parse["atterissage"].take_string().unwrap().as_str(), "%Hh%M").unwrap(),
        }
    }
}