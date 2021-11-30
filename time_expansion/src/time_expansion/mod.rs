use crate::time_expansion::config::ExpansionConfig;
use crate::verilog::Module;

pub mod config;

/// WIP! transition str/stf fault only!
#[derive(Debug)]
pub struct Fault {
    location: String,
    sa_value: bool,
}

#[derive(Debug)]
pub struct ExtractedCombinationalPartModel {
    extracted_module: Module,
    primary_inputs: Vec<String>,
    primary_outputs: Vec<String>,
    pseudo_primary_inputs: Vec<String>,
    pseudo_primary_outputs: Vec<String>,
}

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
