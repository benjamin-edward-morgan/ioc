use std::sync::{Arc, Mutex};
use crate::{RpiGpio,BuildError};
use ioc_core::{Input, InputSource};
use rppal::gpio::{InputPin, Level, Trigger};
use tokio::sync::broadcast;
use serde::Deserialize;
use tracing::error;


#[derive(Deserialize, Debug)]
pub struct DigitalBoolInputConfig {
    pub pin: u8,
    pub pull_up: bool,
}

pub struct DigitalBoolInput {
    state: Arc<Mutex<Level>>,
    rx: broadcast::Receiver<bool>,
    _pin: InputPin, //needs to stay in scope for the async interrupt
}

impl DigitalBoolInput {
    pub fn try_build(rpi_gpio: &RpiGpio, cfg: &DigitalBoolInputConfig) -> Result<Self, BuildError> {

        //get the gpio pin, return an error if we can't
        let pin = rpi_gpio.gpio.get(cfg.pin)?;

        //configure the pin's internal pull up or pull down resistor
        let mut pin = if cfg.pull_up {
            pin.into_input_pullup()
        } else {
            pin.into_input_pulldown()
        };

        //broadcast where we send updates when the voltage changes
        let (tx, rx) = broadcast::channel(rpi_gpio.channel_size as usize);

        //read the current state of the pin, save it. 
        let state = Arc::new(Mutex::new(pin.read()));

        //another reference to the state for the interrupt handler 
        let state_i = state.clone();

        //set interrupt to update state and broadcast new value
        pin.set_async_interrupt(Trigger::Both, move |level| {
            match state_i.lock() {
                Ok(mut state) => {
                    *state = level;

                    if let Err(err) = tx.send(level == Level::High) {
                        //todo: better to panic if we can't write to channel?
                        error!("error sending value in gpio digital in: {}", err);
                    }
                },
                Err(err) => {
                    //todo: better to panic if lock is poisoned?
                    error!("can't get mtx lock in gpio digital in: {}", err);
                },
            }
        })?;
        
        Ok(DigitalBoolInput {
            state,
            rx,
            _pin: pin,
        })
    }
}

impl Input<bool> for DigitalBoolInput {
    fn source(&self) -> InputSource<bool> {
        if let Ok(state) = self.state.lock() {
            InputSource {
                start: (*state == Level::High),
                rx: self.rx.resubscribe(),
            }
        } else {
            panic!("DigitalBoolInput can't acquire the lock the current state in .source()");
        }
    }
}
