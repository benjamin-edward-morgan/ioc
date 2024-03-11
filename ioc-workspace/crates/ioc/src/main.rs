
// #[cfg(feature = "rpi")]
// mod lf;

// #[cfg(feature = "rpi")]
// use lf::littlefoot_main;

// mod dev;

// use dev::dev_main;

mod cfg;

use cfg::cfg_main;


#[tokio::main]
async fn main() {

    // #[cfg(feature = "rpi")] 
    // {
    //     littlefoot_main().await
    // }

    // #[cfg(not(feature = "rpi"))]
    // {
    //     dev_main().await
    // }

    cfg_main().await;

}
