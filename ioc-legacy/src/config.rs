use std::{collections::HashMap, fmt::Display};

use config::{Config, ConfigError, File, Map};
use serde::Deserialize;
use tokio::task::JoinHandle;
use tracing::warn;

use crate::{
    channel::{Channel, ChannelConfig},
    controller::{pid::PidControllerConfig, DirectControllerConfig},
    input::{constant::ConstantInputConfig, noise::NoiseInputConfig},
    output::ConsoleOutputConfig,
    sim::second_order_ode::SecondOrderOdeConfig,
    Input, InputSource, Output, OutputSink,
};

#[cfg(feature = "ws-server")]
use crate::ws::{
    input::WsInput, output::WsOutput, WsInputBoolConfig, WsInputFloatConfig, WsInputStringConfig,
    WsServer, WsServerConfig, WsStateConfig, WsStateInputConfig, WsStateOutputConfig,
};

#[cfg(feature = "rpi")]
use crate::rpi::{
    InputRpiConfig, OutputRpiConfig, Rpi, RpiBuilder, RpiDigitalBoolInputConfig,
    RpiPwmFloatOutputConfig,
};

#[derive(Deserialize, Debug)]
pub struct IocConfig {
    pub inputs: Map<String, InputConfig>,
    pub outputs: Map<String, OutputConfig>,
    pub channels: Map<String, ChannelConfig>,
    pub controllers: Map<String, ControllerConfig>,
}

impl IocConfig {
    pub fn new(config: &str) -> Result<Self, ConfigError> {
        let cfg = Config::builder()
            .add_source(File::with_name(config))
            .build()?;

        cfg.try_deserialize()
    }

    pub async fn start(self) {
        let mut inputs = HashMap::with_capacity(self.inputs.len());
        let mut outputs = HashMap::with_capacity(self.outputs.len());
        let mut channels = HashMap::with_capacity(self.channels.len());

        let mut handles =
            Vec::with_capacity(self.inputs.len() + self.outputs.len() + self.controllers.len());

        let mut input_core_features = HashMap::with_capacity(self.inputs.len());
        let mut output_core_features = HashMap::with_capacity(self.outputs.len());

        #[cfg(feature = "ws-server")]
        let mut input_ws_srv_features = HashMap::with_capacity(self.inputs.len());

        #[cfg(feature = "ws-server")]
        let mut output_ws_srv_features = HashMap::with_capacity(self.outputs.len());

        #[cfg(feature = "rpi")]
        let mut input_rpi_features = HashMap::with_capacity(self.inputs.len());

        #[cfg(feature = "rpi")]
        let mut output_rpi_features = HashMap::with_capacity(self.outputs.len());

        for (k, i) in self.inputs {
            let input_feature: InputConfigFeature = i.into();
            match input_feature {
                InputConfigFeature::Core(core_cfg) => {
                    input_core_features.insert(k, core_cfg);
                }

                #[cfg(feature = "ws-server")]
                InputConfigFeature::WsServer(ws_srv_cfg) => {
                    input_ws_srv_features.insert(k, ws_srv_cfg);
                }

                #[cfg(feature = "rpi")]
                InputConfigFeature::Rpi(rpi_cfg) => {
                    input_rpi_features.insert(k, rpi_cfg);
                }
            }
        }

        for (k, o) in self.outputs {
            let output_feature: OutputConfigFeature = o.into();
            match output_feature {
                OutputConfigFeature::Core(core_cfg) => {
                    output_core_features.insert(k, core_cfg);
                }

                #[cfg(feature = "ws-server")]
                OutputConfigFeature::WsServer(ws_srv_cfg) => {
                    output_ws_srv_features.insert(k, ws_srv_cfg);
                }

                #[cfg(feature = "rpi")]
                OutputConfigFeature::Rpi(rpi_cfg) => {
                    output_rpi_features.insert(k, rpi_cfg);
                }
            }
        }

        for (k, i) in input_core_features {
            inputs.insert(k, i.boxed());
        }

        for (k, o) in output_core_features {
            outputs.insert(k, o.boxed());
        }

        #[cfg(feature = "ws-server")]
        {
            if input_ws_srv_features.is_empty() && output_ws_srv_features.is_empty() {
                warn!("There are no ws inputs or outputs! Running with the ws-server feature enabled is moot.");
            }

            let srv_cfg = WsServerConfig {
                state_config: WsStateConfig {
                    input_configs: input_ws_srv_features,
                    output_configs: output_ws_srv_features,
                    channel_size: 64,
                },
            };

            let server = WsServer::new(srv_cfg).await;
            handles.push(server.handle);

            for (k, i) in server.inputs {
                match i {
                    WsInput::Float { input } => {
                        inputs.insert(k.to_string(), InputBox::Float(Box::new(input)));
                    }
                    WsInput::Bool { input } => {
                        inputs.insert(k.to_string(), InputBox::Bool(Box::new(input)));
                    }
                    WsInput::String { input } => {
                        inputs.insert(k.to_string(), InputBox::String(Box::new(input)));
                    }
                }
            }

            for (k, o) in server.outputs {
                match o {
                    WsOutput::Float { output } => {
                        outputs.insert(k, OutputBox::Float(Box::new(output)));
                    }
                    WsOutput::Bool { output } => {
                        outputs.insert(k, OutputBox::Bool(Box::new(output)));
                    }
                    WsOutput::String { output } => {
                        outputs.insert(k, OutputBox::String(Box::new(output)));
                    }
                }
            }
        }

        #[cfg(feature = "rpi")]
        {
            if input_rpi_features.is_empty() && output_rpi_features.is_empty() {
                warn!("There are no rpi inputs or outputs! Running with the rpi feature enabled is moot.");
            }

            let rpi = RpiBuilder::new(input_rpi_features, output_rpi_features).build();

            for (k, v) in rpi.inputs {
                inputs.insert(k, v);
            }

            for (k, v) in rpi.outputs {
                outputs.insert(k, v);
            }
        }

        for (k, c) in self.channels {
            channels.insert(k, c.into());
        }

        let ports = BoxedPorts {
            inputs,
            outputs,
            channels,
        };

        self.controllers
            .iter()
            .for_each(|(k, c)| match c.try_build(&ports) {
                Ok(handle) => handles.push(handle),
                Err(cbe) => warn!("Error building controller {}: {}", k, cbe),
            });

        futures::future::join_all(handles).await;
    }
}

// named types for all input configs
#[derive(Deserialize, Debug)]
pub enum InputConfig {
    Constant(ConstantInputConfig),
    Noise(NoiseInputConfig),

    #[cfg(feature = "ws-server")]
    WsFloat(WsInputFloatConfig),
    #[cfg(feature = "ws-server")]
    WsBool(WsInputBoolConfig),
    #[cfg(feature = "ws-server")]
    WsString(WsInputStringConfig),

    #[cfg(feature = "rpi")]
    RpiDigitalBool(RpiDigitalBoolInputConfig),
}

//input configs split up by feature
pub enum InputCoreConfig {
    Constant(ConstantInputConfig),
    Noise(NoiseInputConfig),
}

impl InputCoreConfig {
    pub fn boxed(&self) -> InputBox {
        match self {
            InputCoreConfig::Constant(cfg) => InputBox::Float(cfg.into()),
            InputCoreConfig::Noise(cfg) => InputBox::Float(cfg.into()),
        }
    }
}

//InputConfig, but bucketed by feature
pub enum InputConfigFeature {
    Core(InputCoreConfig),

    #[cfg(feature = "ws-server")]
    WsServer(WsStateInputConfig),

    #[cfg(feature = "rpi")]
    Rpi(InputRpiConfig),
}

impl From<InputConfig> for InputConfigFeature {
    fn from(val: InputConfig) -> Self {
        match val {
            InputConfig::Constant(cfg) => InputConfigFeature::Core(InputCoreConfig::Constant(cfg)),
            InputConfig::Noise(cfg) => InputConfigFeature::Core(InputCoreConfig::Noise(cfg)),

            #[cfg(feature = "ws-server")]
            InputConfig::WsFloat(cfg) => {
                InputConfigFeature::WsServer(WsStateInputConfig::Float(cfg))
            }
            #[cfg(feature = "ws-server")]
            InputConfig::WsBool(cfg) => {
                InputConfigFeature::WsServer(WsStateInputConfig::Bool(cfg))
            },
            #[cfg(feature = "ws-server")]
            InputConfig::WsString(cfg) => {
                InputConfigFeature::WsServer(WsStateInputConfig::String(cfg))
            }

            #[cfg(feature = "rpi")]
            InputConfig::RpiDigitalBool(cfg) => {
                InputConfigFeature::Rpi(InputRpiConfig::RpiDigitalBoolInput(cfg))
            }
        }
    }
}

#[derive(Deserialize, Debug)]
pub enum OutputConfig {
    ConsoleFloat(ConsoleOutputConfig),
    ConsoleBool(ConsoleOutputConfig),
    ConsoleString(ConsoleOutputConfig),

    #[cfg(feature = "ws-server")]
    WsFloat,
    #[cfg(feature = "ws-server")]
    WsBool,
    #[cfg(feature = "ws-server")]
    WsString,

    #[cfg(feature = "rpi")]
    RpiPwmFloat(RpiPwmFloatOutputConfig),
}

pub enum OutputCoreConfig {
    ConsoleFloat(ConsoleOutputConfig),
    ConsoleBool(ConsoleOutputConfig),
    ConsoleString(ConsoleOutputConfig),
}

impl OutputCoreConfig {
    pub fn boxed(&self) -> OutputBox {
        match self {
            OutputCoreConfig::ConsoleFloat(cfg) => OutputBox::Float(cfg.into()),
            OutputCoreConfig::ConsoleBool(cfg) => OutputBox::Bool(cfg.into()),
            OutputCoreConfig::ConsoleString(cfg) => OutputBox::String(cfg.into()),
        }
    }
}

pub enum OutputConfigFeature {
    Core(OutputCoreConfig),

    #[cfg(feature = "ws-server")]
    WsServer(WsStateOutputConfig),

    #[cfg(feature = "rpi")]
    Rpi(OutputRpiConfig),
}

impl From<OutputConfig> for OutputConfigFeature {
    fn from(val: OutputConfig) -> Self {
        match val {
            OutputConfig::ConsoleBool(cfg) => {
                OutputConfigFeature::Core(OutputCoreConfig::ConsoleBool(cfg))
            }
            OutputConfig::ConsoleFloat(cfg) => {
                OutputConfigFeature::Core(OutputCoreConfig::ConsoleFloat(cfg))
            }
            OutputConfig::ConsoleString(cfg) => {
                OutputConfigFeature::Core(OutputCoreConfig::ConsoleString(cfg))
            }

            #[cfg(feature = "ws-server")]
            OutputConfig::WsFloat => OutputConfigFeature::WsServer(WsStateOutputConfig::Float),
            #[cfg(feature = "ws-server")]
            OutputConfig::WsBool => OutputConfigFeature::WsServer(WsStateOutputConfig::Bool),
            #[cfg(feature = "ws-server")]
            OutputConfig::WsString => OutputConfigFeature::WsServer(WsStateOutputConfig::String),

            #[cfg(feature = "rpi")]
            OutputConfig::RpiPwmFloat(cfg) => {
                OutputConfigFeature::Rpi(OutputRpiConfig::RpiPwmFloatOutput(cfg))
            }
        }
    }
}

#[derive(Deserialize, Debug)]
pub enum ControllerConfig {
    DirectFloat(DirectControllerConfig),
    DirectBool(DirectControllerConfig),
    DirectString(DirectControllerConfig),

    //Function1()
    PID(PidControllerConfig),
    SimSecondOrder(SecondOrderOdeConfig),
}

impl ControllerBuilder for ControllerConfig {
    fn try_build(&self, ports: &BoxedPorts) -> Result<JoinHandle<()>, ControllerBuilderError> {
        match self {
            ControllerConfig::DirectFloat(config) => config.try_build_float(ports),
            ControllerConfig::DirectBool(config) => config.try_build_bool(ports),
            ControllerConfig::DirectString(config) => config.try_build_string(ports),
            ControllerConfig::PID(config) => config.try_build(ports),
            ControllerConfig::SimSecondOrder(config) => config.try_build(ports),
        }
    }
}

// internals

pub enum OutputBox {
    Float(Box<dyn Output<f64>>),
    Bool(Box<dyn Output<bool>>),
    String(Box<dyn Output<String>>),
}

impl OutputBox {
    pub fn get_float_sink(&self) -> Result<OutputSink<f64>, ControllerBuilderError> {
        match self {
            OutputBox::Float(f) => Ok(f.sink()),
            wrong_box => Err(ControllerBuilderError::from_string(format!(
                "cannot use output {} as Float",
                wrong_box
            ))),
        }
    }
    pub fn get_bool_sink(&self) -> Result<OutputSink<bool>, ControllerBuilderError> {
        match self {
            OutputBox::Bool(b) => Ok(b.sink()),
            wrong_box => Err(ControllerBuilderError::from_string(format!(
                "cannot use output {} as Bool",
                wrong_box
            ))),
        }
    }
    pub fn get_string_sink(&self) -> Result<OutputSink<String>, ControllerBuilderError> {
        match self {
            OutputBox::String(s) => Ok(s.sink()),
            wrong_box => Err(ControllerBuilderError::from_string(format!(
                "cannot use output {} as String",
                wrong_box
            ))),
        }
    }
}

impl Display for OutputBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            OutputBox::Float(_) => "Float",
            OutputBox::Bool(_) => "Bool",
            OutputBox::String(_) => "String",
        };
        f.write_str(str)
    }
}

pub enum InputBox {
    Float(Box<dyn Input<f64>>),
    Bool(Box<dyn Input<bool>>),
    String(Box<dyn Input<String>>),
}

impl InputBox {
    pub fn get_float_source(&self) -> Result<InputSource<f64>, ControllerBuilderError> {
        match self {
            InputBox::Float(f) => Ok(f.source()),
            wrong_box => Err(ControllerBuilderError::from_string(format!(
                "cannot use input {} as a Float",
                wrong_box
            ))),
        }
    }
    pub fn get_bool_source(&self) -> Result<InputSource<bool>, ControllerBuilderError> {
        match self {
            InputBox::Bool(b) => Ok(b.source()),
            wrong_box => Err(ControllerBuilderError::from_string(format!(
                "cannot use input {} as a Bool",
                wrong_box
            ))),
        }
    }
    pub fn get_string_source(&self) -> Result<InputSource<String>, ControllerBuilderError> {
        match self {
            InputBox::String(s) => Ok(s.source()),
            wrong_box => Err(ControllerBuilderError::from_string(format!(
                "cannot use input {} as a String",
                wrong_box
            ))),
        }
    }
}

impl Display for InputBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            InputBox::Float(_) => "Float",
            InputBox::Bool(_) => "Bool",
            InputBox::String(_) => "String",
        };
        f.write_str(str)
    }
}

pub enum ChannelBox {
    Float(Channel<f64>),
    Bool(Channel<bool>),
    String(Channel<String>),
}

impl ChannelBox {
    pub fn get_float_source(&self) -> Result<InputSource<f64>, ControllerBuilderError> {
        match self {
            ChannelBox::Float(f) => Ok(f.source()),
            wrong_box => Err(ControllerBuilderError::from_string(format!(
                "cannot use {} as a Float",
                wrong_box
            ))),
        }
    }
    pub fn get_bool_source(&self) -> Result<InputSource<bool>, ControllerBuilderError> {
        match self {
            ChannelBox::Bool(b) => Ok(b.source()),
            wrong_box => Err(ControllerBuilderError::from_string(format!(
                "cannot use {} as a Bool",
                wrong_box
            ))),
        }
    }
    pub fn get_string_source(&self) -> Result<InputSource<String>, ControllerBuilderError> {
        match self {
            ChannelBox::String(s) => Ok(s.source()),
            wrong_box => Err(ControllerBuilderError::from_string(format!(
                "cannot use {} as a String",
                wrong_box
            ))),
        }
    }

    pub fn get_float_sink(&self) -> Result<OutputSink<f64>, ControllerBuilderError> {
        match self {
            ChannelBox::Float(f) => Ok(f.sink()),
            wrong_box => Err(ControllerBuilderError::from_string(format!(
                "cannot use output {} as Float",
                wrong_box
            ))),
        }
    }
    pub fn get_bool_sink(&self) -> Result<OutputSink<bool>, ControllerBuilderError> {
        match self {
            ChannelBox::Bool(b) => Ok(b.sink()),
            wrong_box => Err(ControllerBuilderError::from_string(format!(
                "cannot use output {} as Bool",
                wrong_box
            ))),
        }
    }
    pub fn get_string_sink(&self) -> Result<OutputSink<String>, ControllerBuilderError> {
        match self {
            ChannelBox::String(s) => Ok(s.sink()),
            wrong_box => Err(ControllerBuilderError::from_string(format!(
                "cannot use output {} as String",
                wrong_box
            ))),
        }
    }
}

impl Display for ChannelBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            ChannelBox::Float(_) => "Float",
            ChannelBox::Bool(_) => "Bool",
            ChannelBox::String(_) => "String",
        };
        f.write_str(str)
    }
}

pub struct BoxedPorts {
    inputs: HashMap<String, InputBox>,
    outputs: HashMap<String, OutputBox>,
    channels: HashMap<String, ChannelBox>,
}

impl BoxedPorts {
    pub fn get_float_source(&self, k: &str) -> Result<InputSource<f64>, ControllerBuilderError> {
        match self.inputs.get(k) {
            Some(input_box) => input_box.get_float_source(),
            None => match self.channels.get(k) {
                Some(channel) => channel.get_float_source(),
                None => Err(ControllerBuilderError::from_string(format!(
                    "Can't find an input or channel with name {}",
                    k
                ))),
            },
        }
    }
    pub fn get_bool_source(&self, k: &str) -> Result<InputSource<bool>, ControllerBuilderError> {
        match self.inputs.get(k) {
            Some(input_box) => input_box.get_bool_source(),
            None => match self.channels.get(k) {
                Some(channel) => channel.get_bool_source(),
                None => Err(ControllerBuilderError::from_string(format!(
                    "Can't find an input or channel with name {}",
                    k
                ))),
            },
        }
    }
    pub fn get_string_source(
        &self,
        k: &str,
    ) -> Result<InputSource<String>, ControllerBuilderError> {
        match self.inputs.get(k) {
            Some(input_box) => input_box.get_string_source(),
            None => match self.channels.get(k) {
                Some(channel) => channel.get_string_source(),
                None => Err(ControllerBuilderError::from_string(format!(
                    "Can't find an input or channel with name {}",
                    k
                ))),
            },
        }
    }

    pub fn get_float_sink(&self, k: &str) -> Result<OutputSink<f64>, ControllerBuilderError> {
        self.outputs
            .get(k)
            .map(|o| o.get_float_sink())
            .or(self.channels.get(k).map(|c| c.get_float_sink()))
            .unwrap_or(Err(ControllerBuilderError::from_string(format!(
                "Can't find an input or channel with name {}",
                k
            ))))
    }
    pub fn get_bool_sink(&self, k: &str) -> Result<OutputSink<bool>, ControllerBuilderError> {
        self.outputs
            .get(k)
            .map(|o| o.get_bool_sink())
            .or(self.channels.get(k).map(|c| c.get_bool_sink()))
            .unwrap_or(Err(ControllerBuilderError::from_string(format!(
                "Can't find an input or channel with name {}",
                k
            ))))
    }
    pub fn get_string_sink(&self, k: &str) -> Result<OutputSink<String>, ControllerBuilderError> {
        self.outputs
            .get(k)
            .map(|o| o.get_string_sink())
            .or(self.channels.get(k).map(|c| c.get_string_sink()))
            .unwrap_or(Err(ControllerBuilderError::from_string(format!(
                "Can't find an input or channel with name {}",
                k
            ))))
    }
}

pub struct ControllerBuilderError {
    pub errors: Vec<String>,
}

impl ControllerBuilderError {
    pub fn new(s: &str) -> Self {
        ControllerBuilderError {
            errors: vec![s.to_string()],
        }
    }
    pub fn from_string(s: String) -> Self {
        ControllerBuilderError { errors: vec![s] }
    }
    pub fn from_vec(errs: Vec<String>) -> Self {
        ControllerBuilderError { errors: errs }
    }
    pub fn from_errors(errs: Vec<ControllerBuilderError>) -> Self {
        let errs: Vec<String> = errs
            .iter()
            .flat_map(|err| err.errors.iter())
            .map(|s| s.to_string())
            .collect();
        ControllerBuilderError::from_vec(errs)
    }
}

impl std::fmt::Display for ControllerBuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ControllerBuilderError\n{}", self.errors.join("\n"))
    }
}

pub trait ControllerBuilder {
    fn try_build(&self, ports: &BoxedPorts) -> Result<JoinHandle<()>, ControllerBuilderError>;
}
