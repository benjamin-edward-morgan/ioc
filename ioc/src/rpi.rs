use std::collections::HashMap;

use serde::Deserialize;
use rppal::gpio::Gpio;
use crate::config::{InputBox, OutputBox};


mod digital;
mod pwm;

#[derive(Deserialize, Debug)]
pub struct RpiDigitalBoolInputConfig {
    pin: u8,
    pull_up: bool,
}

pub enum InputRpiConfig {
    RpiDigitalBoolInput(RpiDigitalBoolInputConfig),
}

impl InputRpiConfig {
    pub fn build(&self, gpio: &Gpio) -> InputBox {
        match self {
            Self::RpiDigitalBoolInput(cfg) => InputBox::Bool(
                Box::new(digital::GpioDigitalBoolInput::new(gpio, cfg))
            ),
        }
    }
}


#[derive(Deserialize, Debug)]
pub struct RpiPwmFloatOutputConfig {
    pin: u8,
    hertz: f64,
}

pub enum OutputRpiConfig {
    RpiPwmFloatOutput(RpiPwmFloatOutputConfig),
}

impl OutputRpiConfig {
    fn build(&self, gpio: &Gpio) -> OutputBox {
        match self {
            Self::RpiPwmFloatOutput(cfg) => OutputBox::Float(
                Box::new(pwm::GpioPwmFloatOutput::new(gpio, cfg))
            ),
        }
    }
}



pub struct RpiBuilder {
    inputs: HashMap<String, InputRpiConfig>,
    outputs: HashMap<String, OutputRpiConfig>,
}

impl RpiBuilder {
    pub fn new(
        inputs: HashMap<String, InputRpiConfig>,
        outputs: HashMap<String, OutputRpiConfig>,
    ) -> Self {
        Self { inputs: inputs, outputs: outputs }
    }

    pub fn build(self) -> Rpi {
        let gpio = Gpio::new().unwrap();

        let mut inputs = HashMap::with_capacity(self.inputs.len());
        let mut outputs = HashMap::with_capacity(self.outputs.len());

        for (k, v) in self.inputs {
            inputs.insert(k, v.build(&gpio));
        }

        for (k, v) in self.outputs {
            outputs.insert(k, v.build(&gpio));
        }

        Rpi { 
            inputs: inputs,
            outputs: outputs,
        }

    }
}

pub struct Rpi {
    pub inputs: HashMap<String, InputBox>,
    pub outputs: HashMap<String, OutputBox>,
}