use crate::{error::BuildError, RpiGpio};
use ioc_core::{Output, OutputSink};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error};

#[derive(Deserialize, Debug)]
pub struct SoftPwmFloatOutputConfig {
    pub pin: u8,
    pub frequency_hertz: f64,
}

pub struct SoftPwmFloatOutput {
    pub handle: JoinHandle<()>,
    tx: mpsc::Sender<f64>,
}

impl SoftPwmFloatOutput {
    pub fn try_build(
        rpi_gpio: &RpiGpio,
        cfg: &SoftPwmFloatOutputConfig,
    ) -> Result<Self, BuildError> {
        //get the gpio pin, return an error if we can't
        let pin = rpi_gpio.gpio.get(cfg.pin)?;

        //convert this pin into an output
        let mut pin = pin.into_output();

        //consumer channel to receive
        let (tx, mut rx) = mpsc::channel(rpi_gpio.channel_size as usize);

        //spawn a task that receives bool updates and sets pin state accordingly
        let frequency_hertz: f64 = cfg.frequency_hertz;
        let handle = tokio::spawn(async move {
            while let Some(new_val) = rx.recv().await {
                if let Err(err) = pin.set_pwm_frequency(frequency_hertz, new_val) {
                    error!("error setting pwm output: {}", err);
                }
            }
            debug!("soft pwm out shutting down");
        });

        Ok(SoftPwmFloatOutput { handle, tx })
    }
}

impl Output<f64> for SoftPwmFloatOutput {
    fn sink(&self) -> OutputSink<f64> {
        OutputSink {
            tx: self.tx.clone(),
        }
    }
}
