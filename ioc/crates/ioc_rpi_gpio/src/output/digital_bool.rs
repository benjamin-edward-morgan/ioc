use crate::{error::BuildError, RpiGpio};
use ioc_core::{Output, OutputSink};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::debug;

#[derive(Deserialize, Debug)]
pub struct DigitalBoolOutputConfig {
    pub pin: u8,
}

pub struct DigitalBoolOutput {
    pub handle: JoinHandle<()>,
    tx: mpsc::Sender<bool>,
}

impl DigitalBoolOutput {
    pub fn try_build(
        rpi_gpio: &RpiGpio,
        cfg: &DigitalBoolOutputConfig,
    ) -> Result<Self, BuildError> {
        //get the gpio pin, return an error if we can't
        let pin = rpi_gpio.gpio.get(cfg.pin)?;

        //convert this pin into an output
        let mut pin = pin.into_output();

        //consumer channel to receive
        let (tx, mut rx) = mpsc::channel(rpi_gpio.channel_size as usize);

        //spawn a task that receives bool updates and sets pin state accordingly
        let handle = tokio::spawn(async move {
            while let Some(new_val) = rx.recv().await {
                if new_val {
                    pin.set_high();
                } else {
                    pin.set_low();
                }
            }
            debug!("digital bool out shutting down");
        });

        Ok(DigitalBoolOutput { handle, tx })
    }
}

impl Output<bool> for DigitalBoolOutput {
    fn sink(&self) -> OutputSink<bool> {
        OutputSink {
            tx: self.tx.clone(),
        }
    }
}
