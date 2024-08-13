use serveur::{data_dir, Configuration};

#[cfg(not(debug_assertions))]
use human_panic::setup_panic;

use std::io::IsTerminal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    //initialisation des outils cli (confy, log, panic)
    let configuration = confy::load("cepo", None).unwrap_or_else(|err| {
        log::error!(
            "Config file not found : {} \nFor information the file should be located at {:?}",
            err,
            data_dir()
        );
        if std::io::stdout().is_terminal() {
            let answer = inquire::Confirm::new("Do you want to write default configuration file ?")
                .with_default(true)
                .prompt();
            match answer {
                Ok(true) => {
                    copy_example_configuration_file();
                }
                Err(_) => println!("Error with questionnaire, try again later"),
                _ => (),
            }
        }
        Configuration::default()
    });
    confy::store("cepo", None, configuration.clone())?;
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(configuration.niveau_log.clone()),
    )
    .init();

    #[cfg(not(debug_assertions))]
    setup_panic!();

    Ok(())
}

mod tests;
