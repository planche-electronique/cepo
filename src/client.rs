pub trait VariationRequete {
    fn incrementer(&mut self, adresse: String);
    fn decrementer(&mut self, adresse: String);
}

#[derive(Clone, PartialEq)]
pub struct Client {
    adresse: String,
    requetes_en_cours: i32,
}

impl VariationRequete for Vec<Client> {
    
    fn incrementer(&mut self, adresse: String) {
        //println!("+1 connection : {}", adresse.clone());
        let mut adresse_existe: bool = false;
        for mut client in self.clone() {
            if client.adresse == adresse {
                if client.requetes_en_cours < 10 {
                    client.requetes_en_cours += 1;
                    adresse_existe = true;
                } else {
                    println!("pas plus de requêtes pour {}", adresse);
                }
            }
            if adresse_existe == false {
                self.push(Client {
                    adresse: adresse.to_string(),
                    requetes_en_cours: 1,
                });
            }
        }
    }
    
    fn decrementer(&mut self, adresse: String) {
        for mut client in self.clone() {
            if client.adresse == adresse {
                if client.requetes_en_cours != 1 {
                    client.requetes_en_cours -= 1;
                } else {
                    let index = self.iter().position(|x| *x == client).unwrap();
                    self.remove(index);
                }
            }
        }
    }
    
}