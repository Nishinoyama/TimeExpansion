use std::fs::File;
use std::io::prelude::*;
use regex::Regex;
use crate::time_expansion::config::ExpansionConfig;

#[derive(Clone, Debug, Default)]
pub struct Verilog {
    top_module: String,
    other_module: Vec<String>,

    input_definitions: Vec<String>,
    output_definitions: Vec<String>,
    wire_definitions: Vec<String>,
    assign_definitions: Vec<String>,
    flipflop_definitions: Vec<String>,
    combination_circuit_definitions: Vec<String>,
}

impl Verilog {
    pub fn from_config(config: ExpansionConfig) -> Verilog {
        let mut verilog = Verilog::default();
        // verilog.top_module = config.get_top_module();
        verilog
    }
}
