use std::sync::{Arc, Mutex};

use tokio::sync::broadcast;
use rppal::gpio::{InputPin,Gpio,Trigger, Level};
use tracing::warn;
use super::RpiDigitalBoolInputConfig;
use crate::{Input,InputSource};

pub struct GpioDigitalBoolInput {
    state: Arc<Mutex<Level>>,
    rx: broadcast::Receiver<bool>,
    pin: InputPin,
}

impl GpioDigitalBoolInput {
    pub fn new(gpio: &Gpio, cfg: &RpiDigitalBoolInputConfig) -> Self {
        
        let pin = gpio.get(cfg.pin).unwrap();

        let mut pin = if cfg.pull_up {
            pin.into_input_pullup()
        } else {
            pin.into_input_pulldown()
        };

        let (tx, rx) = broadcast::channel(16);
        let state = Arc::new(Mutex::new(pin.read()));

        let interrupt_state = state.clone();
        pin.set_async_interrupt(Trigger::Both, move |level| {
            match interrupt_state.lock() {
                Ok(mut s) => {
                    *s = level;

                    if let Err(err) = tx.send(level == Level::High) {
                        warn!("error sending value in gpio digital in: {}", err);
                    }
                },
                Err(err) => {
                    warn!("can't get mtx lock in gpio digital in: {}", err);
                }
            }
        }).unwrap();

        GpioDigitalBoolInput {
            state: state,
            rx: rx,
            pin: pin
        }
    }
}

impl Input<bool> for GpioDigitalBoolInput {
    fn source(&self) -> InputSource<bool> {
        if let Ok(state) = self.state.lock() {
            InputSource { 
                start: (*state == Level::High), 
                rx: self.rx.resubscribe() 
            }
        } else {
            panic!("can't get gpio digital in current state!");
        }
    }
}