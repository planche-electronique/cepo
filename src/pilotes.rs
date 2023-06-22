use json::JsonValue::Array;
use std::fs;

fn lire_liste_pilotes() -> Vec<String> {
    let contenu_fichier = fs::read_to_string("./parametres/pilotes.json").unwrap_or_default();
    let fichier_parse = json::parse(contenu_fichier.as_str()).unwrap();
    let pilotes_json = match fichier_parse {
        Array(vecteur) => vecteur,
        _ => {
            eprintln!("pilotes.json n'est pas un tableau");
            Vec::new()
        }
    };

    let mut pilotes = Vec::new();

    for pilote_json in pilotes_json {
        match pilote_json {
            json::JsonValue::Short(pilote) => {
                pilotes.push(pilote.as_str().to_string());
            }
            _ => {
                eprintln!("{} n'est pas de type short", pilote_json);
            }
        }
    }
    pilotes
}
