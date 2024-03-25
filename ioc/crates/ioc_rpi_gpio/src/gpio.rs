use std::collections::HashMap;

use ioc_core::{error::IocBuildError, Input, InputKind, Module, ModuleIO, Output, OutputKind};
use rppal::gpio::{Level, Trigger};
use serde::Deserialize;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error};
use futures_util::future::join_all;
use crate::error::GpioError;

#[derive(Debug, Deserialize)]
pub enum PinConfig {
    //The produces an output that accepts booleans. The output will set the pin to high when true and low when false.
    DigitalOut{ pin: u8 },
    //This produces an input that emits booleans. The input will read the pin and send true when the pin is high and false when the pin is low.
    DigitalIn{ pin: u8, pull_up: bool },
    //This produces an output that accepts floats between 0.0 and 1.0. The output will set the pin to a pwm signal with the given frequency. The PWM signal is implemented in software and can be noisy.
    SoftPwmOut{ pin: u8, frequency: f64 },
}

//Configuration to create a Gpio module with named pins which can be inputs or outputs.
#[derive(Debug, Deserialize)]
pub struct GpioConfig {
    pub pins: HashMap<String, PinConfig>,
}

pub struct Gpio {
    join_handle: JoinHandle<()>,
    inputs: HashMap<String, InputKind>,
    outputs: HashMap<String, OutputKind>,
}

impl From<Gpio> for ModuleIO {
    fn from(gpio: Gpio) -> Self {
        ModuleIO { 
            join_handle: gpio.join_handle, 
            inputs: gpio.inputs, 
            outputs: gpio.outputs,
        }
    }
}

fn spawn_gpio_digital_in(gpio: &rppal::gpio::Gpio, pin: u8, pull_up: bool) -> Result<Input<bool>, GpioError> {
    let pin = gpio.get(pin)?;

    let mut pin = if pull_up {
        pin.into_input_pullup()
    } else {
        pin.into_input_pulldown()
    };
    let start = pin.read();
    let (input, tx) = Input::new(start == Level::High);
    pin.set_async_interrupt(Trigger::Both, move |level| {
        if let Err(err) = tx.send(level == Level::High) {
            error!("error sending value in gpio digital in: {}", err);
        }
    })?;
    Ok(input)
}

fn spawn_gpio_digital_out(gpio: &rppal::gpio::Gpio, pin: u8) -> Result<(Output<bool>, JoinHandle<()>), GpioError> {
    let pin = gpio.get(pin)?;

    let mut pin = pin.into_output();

    let (output, mut rx) = Output::<bool>::new();

    let handle = tokio::spawn(async move {
        while let Some(new_val) = rx.recv().await {
            let level = if new_val { Level::High } else { Level::Low };
            pin.write(level);
        }
        debug!("gpio digital out shutting down");
        pin.set_low();
    });

    Ok((output, handle))
}

fn spawn_gpio_soft_pwm_out(gpio: &rppal::gpio::Gpio, pin: u8, frequency: f64) -> Result<(Output<f64>, JoinHandle<()>), GpioError> {
    let pin = gpio.get(pin)?;

    let mut pin = pin.into_output();

    let (output, mut rx) = Output::<f64>::new();

    let handle = tokio::spawn(async move {
        while let Some(new_val) = rx.recv().await {
            let duty_cycle = new_val.min(1.0).max(0.0);
            if let Err(err) = pin.set_pwm_frequency(frequency, duty_cycle) {
                error!("error setting pwm output: {}", err);
            }
        }
        debug!("soft pwm out shutting down");
        pin.set_low();
    });

    Ok((output, handle))
}


impl Module for Gpio {
    type Config = GpioConfig;

    async fn try_build(cfg: &GpioConfig, _cancel_token: CancellationToken) -> Result<Self, IocBuildError> {
        let mut inputs = HashMap::new();
        let mut outputs = HashMap::new();
        let mut join_handles = Vec::new();

        let gpio = rppal::gpio::Gpio::new().map_err(|rppal_err| {
            IocBuildError::from_string(format!("error creating gpio: {}", rppal_err))
        })?;

        for (name, pin_cfg) in &cfg.pins {
            match pin_cfg {
                PinConfig::DigitalIn { pin, pull_up } => {
                    let input = spawn_gpio_digital_in(&gpio, *pin, *pull_up)?;
                    inputs.insert(name.clone(), InputKind::Bool(input));
                },
                PinConfig::DigitalOut { pin } => {
                    let (output, handle) = spawn_gpio_digital_out(&gpio, *pin)?;
                    outputs.insert(name.clone(), OutputKind::Bool(output));
                    join_handles.push(handle);
                },
                PinConfig::SoftPwmOut { pin, frequency } => {
                    let (output, handle) = spawn_gpio_soft_pwm_out(&gpio, *pin, *frequency)?;
                    outputs.insert(name.clone(), OutputKind::Float(output));
                    join_handles.push(handle);
                },
            }
        }

        let join_handle = tokio::spawn(async move {
            join_all(join_handles).await;
            debug!("gpio tasks all done!");
        });

        Ok(Self { join_handle, inputs, outputs })
    }
}
