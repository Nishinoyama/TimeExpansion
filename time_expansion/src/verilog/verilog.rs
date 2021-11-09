use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use regex::Regex;
use crate::time_expansion::config::ExpansionConfig;
use crate::verilog::ast::*;
use crate::verilog::ast::token::Lexer;

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
    fn from_file(file_name: String) -> std::io::Result<Verilog> {
        let verilog_file = File::open(file_name)?;
        let mut verilog_buf_reader = BufReader::new(verilog_file);
        let mut verilog_string = String::new();
        for line in verilog_buf_reader.lines() {
            let line = line.unwrap().split("//").next().unwrap().to_string();
            verilog_string += &line;
            verilog_string += &String::from("\n");
        }
        let mut verilog = Lexer::from_chars(verilog_string.chars().clone());
        let tokens = verilog.tokenize();
        Ok(Verilog::default())
    }

    pub fn from_config(config: &ExpansionConfig) -> Self {
        // let mut verilog = Verilog::default();
        eprintln!("{}", config.get_input_file());
        let verilog = Verilog::from_file(config.get_input_file().clone()).unwrap();
        // verilog.top_module = config.get_top_module();
        eprintln!("{:?}", verilog);
        // verilog
        Verilog::default()
    }
}

#[cfg(test)]
mod test {
    use std::fs::File;
    use crate::time_expansion::config::ExpansionConfig;
    use crate::time_expansion::config::ExpansionMethod::Broadside;
    use crate::time_expansion::ff_definition::FFDefinition;
    use crate::time_expansion::inv_definition::InvDefinition;
    use crate::verilog::verilog::Verilog;

    #[test]
    fn expansion_config() {
        let ec = ExpansionConfig::from_file("expansion_example.conf").unwrap();
        let verilog = Verilog::from_config(&ec);
        eprintln!("{:?}", verilog);
    }
}
