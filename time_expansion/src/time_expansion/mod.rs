use crate::time_expansion::config::ExpansionConfig;
use crate::verilog::fault::Fault;
use crate::verilog::{Module, Verilog, Wire};

pub mod config;

#[derive(Debug)]
pub struct ConfiguredModel {
    cfg: ExpansionConfig,
}

impl ConfiguredModel {
    pub fn new(cfg: ExpansionConfig) -> Self {
        Self { cfg }
    }
    pub fn cfg(&self) -> &ExpansionConfig {
        &self.cfg
    }
}

#[derive(Debug)]
pub struct ExtractedCombinationalPartModel {
    extracted_module: Module,
    primary_inputs: Vec<Wire>,
    primary_outputs: Vec<Wire>,
    pseudo_primary_inputs: Vec<Wire>,
    pseudo_primary_outputs: Vec<Wire>,
}

impl From<ConfiguredModel> for ExtractedCombinationalPartModel {
    fn from(cm: ConfiguredModel) -> Self {
        let module = Verilog::from(cm.cfg().clone())
            .module_by_name(cm.cfg().top_module())
            .cloned()
            .unwrap();
        let (extracted_module, pseudo_primary_inputs, pseudo_primary_outputs) =
            cm.cfg().extract_combinational_part(&module);
        let primary_inputs = extracted_module
            .inputs()
            .into_iter()
            .filter(|wire| !pseudo_primary_inputs.contains(wire))
            .cloned()
            .collect();
        let primary_outputs = extracted_module
            .outputs()
            .into_iter()
            .filter(|wire| !pseudo_primary_outputs.contains(wire))
            .cloned()
            .collect();
        Self {
            extracted_module,
            pseudo_primary_inputs,
            pseudo_primary_outputs,
            primary_inputs,
            primary_outputs,
        }
    }
}

pub trait TimeExpansionModel {
    fn primary_inputs(&self) -> Vec<String>;
    fn primary_outputs(&self) -> Vec<String>;
    fn pseudo_primary_inputs(&self) -> Vec<String>;
    fn pseudo_primary_outputs(&self) -> Vec<String>;
}
pub trait TimeExpansionATPGModel {}

#[derive(Debug)]
pub struct BroadSideExpansionModel {
    combinational_module: ExtractedCombinationalPartModel,
    use_primary_io: bool,
}

#[derive(Debug)]
pub struct BroadSideExpansionATPGModel {
    bs_model: BroadSideExpansionModel,
    fault: Fault,
}

#[derive(Debug)]
pub struct BroadSideDiExpansionModel {
    expansion_model: BroadSideExpansionModel,
}

#[derive(Debug)]
pub struct BroadSideDiExpansionATPGModel {
    bsd_model: BroadSideDiExpansionModel,
    faults: Vec<Fault>,
}

#[cfg(test)]
mod test {
    use crate::time_expansion::config::ExpansionConfig;
    use crate::time_expansion::{ConfiguredModel, ExtractedCombinationalPartModel};
    use crate::verilog::netlist_serializer::NetlistSerializer;

    #[test]
    fn configured_model() -> Result<(), String> {
        let cm = ConfiguredModel::new(ExpansionConfig::from_file("expansion_example.conf")?);
        let ec = ExtractedCombinationalPartModel::from(cm);
        eprintln!("{}", ec.extracted_module.gen());
        eprintln!("{:?}", ec.primary_inputs);
        eprintln!("{:?}", ec.primary_outputs);
        eprintln!("{:?}", ec.pseudo_primary_inputs);
        eprintln!("{:?}", ec.pseudo_primary_outputs);
        Ok(())
    }
}
