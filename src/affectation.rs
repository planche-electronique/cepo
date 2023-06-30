#[derive(Debug, Clone, PartialEq)]
struct Affectation {
    pilote_tr: String,   // parmi pilotes_tr
    treuil: String,      // parmi treuils
    pilote_rq: String,   // parmi pilotes_rq
    remorqueurs: String, // parmi remorqueurs
    chef_piste: String,  // parmi pilotes
    affectations: Vec<Vol>,
}
