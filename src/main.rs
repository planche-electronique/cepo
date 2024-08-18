use serveur::{
    configuration::{copy_example_configuration_file, Configuration},
    data_dir, Context,
};

#[cfg(not(debug_assertions))]
use human_panic::setup_panic;

use std::io::IsTerminal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // initializing cli tools (confy, log, panic)
    #[cfg(not(debug_assertions))]
    setup_panic!();

    let mut configuration = confy::load("cepo", None).unwrap_or_else(|err| {
        log::error!(
            "Error while loading configuration : {} \nFor information the file should be located at {:?}",
            err,
            data_dir()
        );
        Configuration::default()
    });
    if configuration == Configuration::default() {
        if std::io::stdout().is_terminal() {
            let answer =
                inquire::Confirm::new("Do you want to write and use example configuration file ?")
                    .with_default(true)
                    .prompt();
            match answer {
                Ok(true) => {
                    log::info!("Writing example configuration file");
                    log::info!("Using example configuration file");
                    copy_example_configuration_file().unwrap();
                    configuration = Configuration::example();
                }
                Err(_) => {
                    log::error!("Error with questionnaire, try again later");
                    panic!();
                }
                _ => (),
            }
        } else {
            panic!(); //should return a more sexy error
        }
    }
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(&configuration.log_level),
    )
    .init();

    let context = Context::new(configuration).await;
    context.server().await?;

    return Ok(());
}

mod tests;
