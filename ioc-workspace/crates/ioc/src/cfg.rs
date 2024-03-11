use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;

use ioc_core::pipe::Pipe;
use ioc_core::transformer::SumConfig;
use ioc_core::transformer::SumInput;
use ioc_core::InputKind;
use ioc_core::OutputKind;
use ioc_core::Input;
use ioc_core::ModuleIO;
use ioc_core::ModuleBuilder;
use ioc_core::{Transformer,TransformerI};
use ioc_extra::input::noise::NoiseInput;
use ioc_extra::input::noise::NoiseInputConfig;
use tracing::{info,error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use serde::Deserialize;
use std::fmt::Debug;
use config::{Config, File, Value};
use futures_util::future::join_all;
use ioc_core::transformer::Sum;

use ioc_server::Server;

use ioc_server::ServerConfig;

#[derive(Deserialize,Debug)]
pub struct IocConfigMetadata {
    pub description: String
}

#[derive(Deserialize,Debug)]
pub enum IocModuleConfig {
    //ioc_server 
    Server(ServerConfig),

    //ioc_extra 
    Noise(NoiseInputConfig),
}


impl IocModuleConfig {
    pub async fn build(&self) -> ModuleIO {
       match self {
            Self::Server(server_config) => {
                let server = Server::try_build(&server_config).await.unwrap();
                server.into()
            }
            Self::Noise(noise_config) => {
                let noise = NoiseInput::try_build(&noise_config).await.unwrap();
                noise.into()
            }
        }
    }
}



#[derive(Deserialize,Debug)]
pub struct SumTransformerConfig{
    inputs: Vec<String>
}


#[derive(Deserialize,Debug)]
pub enum IocTransformer {
    //core
    Sum(SumTransformerConfig),
}

impl IocTransformer {
    async fn build_from_inputs (&self, inputs: &HashMap<String, InputKind>) -> TransformerI {
        match self {
            Self::Sum(sum_cfg) => {
                let sum_inputs = &sum_cfg.inputs;
                let inputs: Vec<_> = sum_inputs.iter().map(|input_key| {
                    match inputs.get(input_key).unwrap() {
                        InputKind::Float(float) => float.as_ref(),
                        x => panic!("Bad input kind to Sum. Expected Float but got {:?}", x)
                    }
                }).collect();

                let cfg = SumConfig{
                    inputs: inputs
                };
                let sum = Sum::try_build(&cfg).await.unwrap();

                TransformerI{
                    inputs: HashMap::from([
                        ("value".to_owned(), InputKind::Float(Box::new(sum.value))),
                    ])
                }
            }
        }
    }

    fn needs_inputs(&self) -> HashSet<&String> {
        match self {
            Self::Sum(sum_cfg) => {
                sum_cfg.inputs.iter().collect()
            }
        }
    }
}

#[derive(Deserialize,Debug)]
pub struct PipeConfig{
    pub from: String,
    pub to: String,
}

#[derive(Deserialize,Debug)]
pub struct IocConfig {
    metadata: IocConfigMetadata,
    modules: config::Map<String, IocModuleConfig>,
    transformers: config::Map<String, IocTransformer>,
    pipes: Vec<PipeConfig>,
}

impl IocConfig {
    pub async fn build(self) {

        let mut modules = HashMap::with_capacity(128);
        let mut handles = Vec::with_capacity(128);
        let mut inputs = HashMap::with_capacity(128);
        let mut outputs = HashMap::with_capacity(128);

        info!("building modules ...");
        for (module_key, module) in self.modules.iter() {
            info!("building module {} ...", module_key);

            let module = module.build().await;
            modules.insert(module_key, module);
        }

        for (module_key, module) in modules {
            for (input_key, input) in module.inputs {
                inputs.insert(
                    format!("{}.{}", module_key, input_key),
                    input,
                );
            }

            for (output_key, output) in module.outputs {
                outputs.insert(
                    format!("{}.{}", module_key, output_key),
                    output 
                );
            }

            handles.push(module.join_handle);
        }
        info!("got the following inputs from modules: {:?}", inputs.keys());
        info!("got the following outputs from modules: {:?}", outputs.keys());
        

        info!("building transformers ...");
        let mut remaining_transformers = self.transformers;

        while !remaining_transformers.is_empty() {
            let mut proccessed_transformers = HashSet::with_capacity(remaining_transformers.len());

            for (transformer_key, transformer) in remaining_transformers.iter() {
                let input_keys: HashSet<&String> = inputs.keys().collect();
                let needs_inputs = transformer.needs_inputs();
                info!("transformer {} needs inputs {:?}", transformer_key, needs_inputs);
                if needs_inputs.is_subset(&input_keys) {
                    info!("building transformer {}", transformer_key);
                    let xformer = transformer.build_from_inputs(&inputs).await;
                    proccessed_transformers.insert(transformer_key.clone());

                    for (xformed_input_key, xformed_input) in xformer.inputs {
                        inputs.insert(
                            format!("{}.{}", transformer_key, xformed_input_key), 
                            xformed_input
                        );
                    }
                } else {
                    info!("skipping transformer {}", transformer_key);
                }
            }

            if proccessed_transformers.is_empty() {
                break;
            } else {
                remaining_transformers.retain(|k, _| {
                    !proccessed_transformers.contains(k)
                });
            }
        }

        info!("have the following inputs after building transformers: {:?}", inputs.keys());
        
        if !remaining_transformers.is_empty() {
            panic!("Unable to build all transformers! Remaining: {:?}", remaining_transformers.keys());


        } 

        info!("building pipes ...");
        for p in self.pipes {
            let input = inputs.get(&p.from).unwrap();
            let output = outputs.get(&p.to).unwrap();
            match (input, output) {
                (InputKind::String(input), OutputKind::String(output)) => {
                    let pipe = Pipe::new(input.as_ref(), output.as_ref());
                },
                (InputKind::Binary(input), OutputKind::Binary(output)) => {
                    let pipe = Pipe::new(input.as_ref(), output.as_ref());
                },
                
                (InputKind::Float(input), OutputKind::Float(output)) => {
                    let pipe = Pipe::new(input.as_ref(), output.as_ref());
                },
                (InputKind::Bool(input), OutputKind::Bool(output)) => {
                    let pipe = Pipe::new(input.as_ref(), output.as_ref());
                },
                (inp, out) => {
                    panic!("got mismatched types when trying to build pipe: {:?}, {:?}", inp, out);
                }
            }
        }

        let all = join_all(handles);
        let results = all.await;
        info!("IOC done! {:?}", results);


    }
}



pub async fn cfg_main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ioc=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cfg = 
        Config::builder()
            .add_source(File::with_name("config"))
            .build()
            .unwrap();
    
    let ioc_cfg: IocConfig = cfg.try_deserialize().unwrap();

    ioc_cfg.build().await;

}