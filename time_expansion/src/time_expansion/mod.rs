use crate::gen_configured_trait;
use crate::time_expansion::config::{
    ConfiguredTrait, ExpansionConfig, ExpansionConfigError, FFDefinition,
};
use crate::verilog::{Gate, Module, Verilog, Wire};
use std::convert::TryFrom;

pub mod config;
pub mod di_expansion_model;
pub mod time_expansion_model;

pub trait TopModule {
    fn top_module(&self) -> &Module;
}

#[derive(Debug, Clone)]
pub struct ConfiguredModel {
    cfg: ExpansionConfig,
    verilog: Verilog,
}
impl ConfiguredModel {
    pub fn cfg(&self) -> &ExpansionConfig {
        &self.cfg
    }
    pub fn verilog(&self) -> &Verilog {
        &self.verilog
    }
    pub fn top_module(&self) -> &Module {
        self.verilog.module_by_name(self.cfg.top_module()).unwrap()
    }

    fn extract_ff_gates(&self) -> Vec<(&FFDefinition, Wire, Gate)> {
        self.top_module()
            .gates()
            .iter()
            .filter_map(|(s, g)| {
                self.cfg()
                    .ff_definitions()
                    .iter()
                    .find(|ff_def| g.name().eq(ff_def.name()))
                    .map(|ff_type| (ff_type, Wire::new_single(s.clone()), g.clone()))
            })
            .collect()
    }
}
gen_configured_trait!(ConfiguredModel, cfg);
impl TopModule for ConfiguredModel {
    fn top_module(&self) -> &Module {
        self.verilog.module_by_name(self.cfg_top_module()).unwrap()
    }
}
impl TryFrom<ExpansionConfig> for ConfiguredModel {
    type Error = ExpansionConfigError;
    fn try_from(cfg: ExpansionConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            verilog: Verilog::try_from(cfg.clone())?,
            cfg,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ExtractedCombinationalPartModel {
    configured_model: ConfiguredModel,
    extracted_module: Module,
    primary_inputs: Vec<Wire>,
    primary_outputs: Vec<Wire>,
    pseudo_primary_inputs: Vec<Wire>,
    pseudo_primary_outputs: Vec<Wire>,
    pseudo_primary_ios: Vec<(Wire, Wire)>,
}
impl ExtractedCombinationalPartModel {
    pub fn configured_model(&self) -> &ConfiguredModel {
        &self.configured_model
    }
    pub fn extracted_module(&self) -> &Module {
        &self.extracted_module
    }
    pub fn primary_inputs(&self) -> &Vec<Wire> {
        &self.primary_inputs
    }
    pub fn primary_outputs(&self) -> &Vec<Wire> {
        &self.primary_outputs
    }
    pub fn pseudo_primary_inputs(&self) -> &Vec<Wire> {
        &self.pseudo_primary_inputs
    }
    pub fn pseudo_primary_outputs(&self) -> &Vec<Wire> {
        &self.pseudo_primary_outputs
    }
    pub fn pseudo_primary_ios(&self) -> &Vec<(Wire, Wire)> {
        &self.pseudo_primary_ios
    }
}
impl TopModule for ExtractedCombinationalPartModel {
    fn top_module(&self) -> &Module {
        self.configured_model.top_module()
    }
}
gen_configured_trait!(ExtractedCombinationalPartModel, configured_model);
impl From<ConfiguredModel> for ExtractedCombinationalPartModel {
    /// Generates combinational part from `self` top [`Module`], their Pseudo Inputs' and Pseudo Outputs' port name.
    /// FFs', in the module, inputs and outputs become the extracted combinational module's inputs and outputs respectively.
    /// Such inputs and outputs are called Pseudo Inputs/Outputs
    fn from(configured_model: ConfiguredModel) -> Self {
        let module = configured_model.top_module();
        let mut extracted_module = module.clone();
        // removes clock/reset defined by config.
        for clock in configured_model.cfg_clock_pins().iter().cloned() {
            extracted_module.remove_input(&Wire::new_single(clock));
        }
        // removes test_s[ies] designed by full scan.
        for test_s_pin in module
            .pins()
            .into_iter()
            .filter(|pin| pin.name().contains("test_s"))
        {
            extracted_module.remove_input(test_s_pin);
            extracted_module.remove_output(test_s_pin);
        }

        let primary_inputs = extracted_module.inputs().iter().cloned().collect();
        let primary_outputs = extracted_module.outputs().iter().cloned().collect();
        let mut pseudo_primary_inputs = Vec::new();
        let mut pseudo_primary_outputs = Vec::new();
        let mut pseudo_primary_ios = Vec::new();
        // extracts the circuit's ffs and their PI/POs will be enumerated PPI/PPOs of extracted_module
        configured_model
            .extract_ff_gates()
            .into_iter()
            .enumerate()
            .for_each(|(i, (ff_def, wire, ff_gate))| {
                let ppi = Wire::new_single(format!("ppi_{}_{}", i + 1, wire.name()));
                let ppo = Wire::new_single(format!("ppo_{}_{}", i + 1, wire.name()));
                // ff's output will be top ppo
                for port in ff_def.data_in().iter() {
                    if let Some(port_wire) = ff_gate.port_by_name(port) {
                        extracted_module.push_assign(format!(
                            "{ppo} = {wire_from_port}",
                            ppo = ppo.name(),
                            wire_from_port = port_wire.wire(),
                        ));
                        extracted_module.push_output(ppo.clone());
                    }
                }
                // ff's input will be top ppo
                for port in ff_def.data_out().iter() {
                    if let Some(port_wire) = ff_gate.port_by_name(port) {
                        extracted_module.push_input(ppi.clone());
                        // if ff's output is inverted (such as QN)
                        if port.contains('N') {
                            extracted_module.push_gate(
                                format!("UN{}", i + 1),
                                configured_model
                                    .cfg_inv_definition()
                                    .to_gate(ppi.name().to_string(), port_wire.wire().to_string()),
                            );
                        } else {
                            extracted_module.push_assign(format!(
                                "{} = {}",
                                port_wire.wire(),
                                ppi.name()
                            ));
                        }
                    }
                }
                extracted_module.remove_gate(wire.name());

                pseudo_primary_inputs.push(ppi.clone());
                pseudo_primary_outputs.push(ppo.clone());
                pseudo_primary_ios.push((ppi, ppo));
            });
        Self {
            configured_model,
            extracted_module,
            pseudo_primary_inputs,
            pseudo_primary_outputs,
            primary_inputs,
            primary_outputs,
            pseudo_primary_ios,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::time_expansion::config::{ExpansionConfig, ExpansionConfigError};
    use crate::time_expansion::{ConfiguredModel, ExtractedCombinationalPartModel};
    use crate::verilog::netlist_serializer::NetlistSerializer;
    use std::convert::TryFrom;

    fn test_configured_model() -> Result<ConfiguredModel, ExpansionConfigError> {
        ConfiguredModel::try_from(ExpansionConfig::from_file("expansion.conf")?)
    }

    #[test]
    fn configured_model() -> Result<(), ExpansionConfigError> {
        let cm = test_configured_model()?;
        eprintln!("{}", cm.verilog.gen());
        Ok(())
    }

    #[test]
    pub fn extract_combinational_part() -> Result<(), ExpansionConfigError> {
        let ecpm = ExtractedCombinationalPartModel::from(test_configured_model()?);
        eprintln!("{}", ecpm.extracted_module.gen());
        eprintln!("{:?}", ecpm.primary_inputs);
        eprintln!("{:?}", ecpm.primary_outputs);
        eprintln!("{:?}", ecpm.pseudo_primary_inputs);
        eprintln!("{:?}", ecpm.pseudo_primary_outputs);
        Ok(())
    }
}
