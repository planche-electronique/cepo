use std::net::TcpStream;
use chrono::{DateTime, NaiveDate, StrftimeItems, prelude::*};

struct Vol {
    numero: i32,
    planeur: String,
    pilote_1: String,
    pilote_2: String,
    code_vol: String,
    code_deco: String,
    machine_deco: String,
    heure_decolage: u8,
    minute_decolage: u8,
    heure_atterissage: u8,
    minute_atterissage: u8,
}

async fn requete_ogn() -> String {
    let jour = 23;
    let mois = 04;
    let annee = 2023;
    let airfield_code = "LFLE";
    let reponse = reqwest::get(format!("http://flightbook.glidernet.org/api/logbook/{}/{}-{}-{}", airfield_code, annee, mois, jour)).await?;
    let corps = reponse.text().await?;
    corps
}