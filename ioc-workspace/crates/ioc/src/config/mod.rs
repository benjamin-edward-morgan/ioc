pub mod module;
pub mod transformer;
pub mod pipe;

use std::collections::{HashMap, HashSet};

use module::IocModuleConfig;
use transformer::IocTransformerConfig;
use pipe::PipeConfig;

use ioc_core::error::IocBuildError;
use serde::Deserialize;
use futures_util::future::join_all;
use tracing::{debug, trace};

#[derive(Deserialize,Debug)]
pub struct IocMetadataConfig {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Deserialize,Debug)]
pub struct IocConfig {
    pub metadata: IocMetadataConfig,
    pub modules: config_rs::Map<String, IocModuleConfig>,
    pub transformers: config_rs::Map<String, IocTransformerConfig>,
    pub pipes: Vec<PipeConfig>,
}

impl IocConfig {
    pub async fn start(self) -> Result<(), IocBuildError> {

        let mut handles = Vec::with_capacity(128);
        let mut inputs = HashMap::with_capacity(128);
        let mut outputs = HashMap::with_capacity(128);

        //build modules, which are collections of inputs and outputs 
        debug!("building modules ...");
        for (module_key, module_config) in self.modules {
            trace!("building module {} ...", module_key);
            match module_config.build().await {
                Ok(module) => {
                    handles.push(module.join_handle);
                    //inputs and outputs are created, prefixed with the module's key
                    for (input_key, input) in module.inputs {
                        inputs.insert(format!("{}.{}", module_key, input_key), input);
                    }
                    for (output_key, output) in module.outputs {
                        outputs.insert(format!("{}.{}", module_key, output_key), output);
                    }
                },
                Err(err) => {
                    return Err(
                        IocBuildError::from_string(format!("Error building module {}: {:?}", module_key, err))
                    );
                }
            }
        }
        
        /* build transformers, which take one or more inputs and create one or more new inputs 
        transformers can be composed. Since they are serialized, we search for transformers 
        need inputs we already have. We build those transformers and add their inputs to the 
        set of all inputs. We iterate along all remaining transformers until they are all built.
        */
        debug!("done bulding modules. building transformers ...");
        let mut remaining_xformers = self.transformers;
        while !remaining_xformers.is_empty() {
            let mut processed_xformers = HashSet::new();

            for (xformer_key, xformer_config) in &remaining_xformers {
                let input_keys: HashSet<&String> = inputs.keys().collect();
                let needs_inputs = xformer_config.needs_inputs();
                if needs_inputs.is_subset(&input_keys) {
                    trace!("building transformer {} ...", xformer_key);
                    match xformer_config.try_build(&inputs).await {
                        Ok(xformer) => {
                            processed_xformers.insert(xformer_key.clone());
                            handles.push(xformer.join_handle);
                            //new inputs are prefixed with the transformer's key
                            for (input_key, input) in xformer.inputs {
                                inputs.insert(format!("{}.{}", xformer_key, input_key), input);
                            }
                        },
                        Err(err) => {
                            return Err(
                                IocBuildError::from_string(format!("failed to build transformer {}: {:?}", xformer_key, err))
                            );
                        },
                    }
                }
            }

            if processed_xformers.is_empty() {
                /* however if we make a pass where no transformers were processed, 
                then we break here are return an error so we don't loop forever 
                */
                break;
            } else {
                //remove those transformer configs that we processed in this pass
                for k in processed_xformers {
                    remaining_xformers.remove(&k);
                }
            }
        }

        if !remaining_xformers.is_empty() {
            //if there are unbuilt transformers, there was likely a missing key. 
            //return an error to the developer can track down their typo
            let known_keys = HashSet::from_iter(inputs.keys());
            let transformer_errors: Vec<String> = remaining_xformers.iter().map(|(k, trans)| {
                let trans_inputs = trans.needs_inputs();
                let missing_inputs: Vec<_> = trans_inputs.difference(&known_keys).into_iter().collect();
                format!("- {}: unable to find inputs: {:?}", k, missing_inputs)
            }).collect();
            return Err(
                IocBuildError::from_string(
                    format!("Unable to build all transformers because not all inputs were found. The following transformers could not be built:\n{}", transformer_errors.join("\n"))
                )
            );
        } 

        //build pipes, which read from a single input and write to a single output
        debug!("done building transformers. building pipes ...");
        for pipe_config in self.pipes {
            trace!("building pipe {:?}", pipe_config);
            let pipe = pipe_config.try_build(&inputs, &outputs)?;
            handles.push(pipe.handle);
        }
        debug!("done bulding pipes. done starting up.");

        //wait for server to exit and all tasks to stop
        join_all(handles).await;

        Ok(())
    }
}