use ioc::config::IocConfig;
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

    if args.len() > 2 {
        println!("only one parameter, the config file, is expected. If omitted defaults to ioc.yml");
        println!("got {}", args.join(","));
    } else {
        let cfg_name: &str = args.get(1).map(|o| o.as_str()).unwrap_or("ioc");
        match IocConfig::new(cfg_name) {
            Ok(cfg) => {
               let ioc = cfg.start();
               ioc.await;
            },
            Err(err) => {
                println!("config error! {}", err);
            }
        }
    }
    

}