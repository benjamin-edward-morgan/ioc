pub mod config;

use config::IocConfig;
use config_rs::{Config, File};
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

///application enrty point
#[tokio::main]
async fn main() {
    //set up logging 
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ioc=debug,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    //get config file name from arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        warn!("only one parameter, the config file, is expected.");
        warn!("got {}", args.join(","));
    } else if let Some(cfg_name) = args.get(1).map(|o| o.as_str()) {

        //try to parse that config file
        let config_res = Config::builder()
            .add_source(File::with_name(cfg_name))
            .build()
            .and_then(|config| config.try_deserialize::<IocConfig>());

        match config_res {
            Ok(config) => {
                //try to start up if we parsed the config
                info!("IOC starting up!");
                if let Some(ref name) = config.metadata.name {
                    info!("name: {name}")
                }
                if let Some(ref descrip) = config.metadata.description {
                    info!("description: {descrip}")
                }

                //this starts the application and waits for it to finish
                match config.start().await {
                    Ok(_) => info!("IOC shut down!"),
                    Err(err) => error!("IOC exited with an error: {:?}", err),
                }
            }
            Err(err) => {
                error!(
                    "Error starting IOC server. Failed to parse config: {:?}",
                    err
                );
            }
        }
    }
}
