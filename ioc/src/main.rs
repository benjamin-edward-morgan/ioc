use ioc::config::IocConfig;
use tracing::{warn, error};
use std::env; 
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};


#[tokio::main]
async fn main() {

    tracing_subscriber::registry()
    .with(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "ioc=info,tower_http=info".into()),
    )
    .with(tracing_subscriber::fmt::layer())
    .init();

    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        warn!("only one parameter, the config file, is expected.");
        warn!("got {}", args.join(","));
    } else {
        if let Some(cfg_name) = args.get(1).map(|o| o.as_str()) {
            match IocConfig::new(cfg_name) {
                Ok(cfg) => {
                   let ioc = cfg.start();
                   ioc.await;
                },
                Err(err) => {
                    error!("config error! {}", err);
                }
            }
        }
    }
    

}