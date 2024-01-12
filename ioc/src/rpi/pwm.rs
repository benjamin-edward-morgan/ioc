use rppal::gpio::Gpio;

use tokio::{sync::mpsc, task::JoinHandle};
use crate::{Output, OutputSink};

use super::RpiPwmFloatOutputConfig;

pub struct GpioPwmFloatOutput {
    tx: mpsc::Sender<f64>,
    handle: JoinHandle<()>,
}

impl GpioPwmFloatOutput {
    pub fn new(gpio: &Gpio, cfg: &RpiPwmFloatOutputConfig) -> Self {
        let mut pin = gpio.get(cfg.pin).unwrap().into_output();
        let (tx, mut rx) = mpsc::channel(16);
        let hertz = cfg.hertz;
        let handle = tokio::spawn(async move {
            while let Some(new_value) = rx.recv().await {
                pin.set_pwm_frequency(hertz, new_value).unwrap();
            }
        });

        GpioPwmFloatOutput { 
            tx: tx,
            handle: handle
        }
    }
}

impl Output<f64> for GpioPwmFloatOutput {
    fn sink(&self) -> OutputSink<f64> {
        OutputSink { 
            tx: self.tx.clone() 
        }
    }
}