use crate::verilog::fault::Fault;
use crate::verilog::Module;

pub mod config;

#[derive(Debug)]
pub struct ExtractedCombinationalPartModel {
    extracted_module: Module,
    primary_inputs: Vec<String>,
    primary_outputs: Vec<String>,
    pseudo_primary_inputs: Vec<String>,
    pseudo_primary_outputs: Vec<String>,
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
