pub mod config;

use config::IocConfig;
use config_rs::{Config, File};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

///application entry point
#[tokio::main]
async fn main() {
    //set up logging 
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ioc=info,tower_http=info".into()),
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

                let cancel_token = get_cancellation_token();

                //this starts the application and waits for it to finish
                match config.start(cancel_token).await {
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


fn get_cancellation_token() -> CancellationToken {
    let token = CancellationToken::new();
    let task_token = token.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for ctrl-c");
        info!("ctrl-c received, shutting down");
        task_token.cancel();
    });
    token
}