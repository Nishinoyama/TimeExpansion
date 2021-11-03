use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::option::Option::Some;
use std::process::exit;
use regex::Regex;
use crate::time_expansion::config::ExpansionMethods::{Broadside, SkewedLoad};

#[derive(Clone, Default)]
pub struct ExpansionConfig {
    expand_method: ExpansionMethods,
    input_file: String,
    output_file: String,
    top_module: String,

    clock_pins: Vec<String>,

    use_primary_io: bool,
    equivalent_check: String,
    ff_definitions: Vec<String>,

    inv_type: String,
    inv_input: String,
    inv_output: String,
}

impl ExpansionConfig {
    pub fn from_file( file_name: &str ) -> std::io::Result<ExpansionConfig> {
        let config_file = File::open(file_name)?;
        let mut config_buf_reader = BufReader::new(config_file);
        let mut lines = Vec::new();
        for line in config_buf_reader.lines() {
            let line = line.unwrap().split("#").next().unwrap().to_string();
            lines.push(line);
        }

        let mut config = Self::default();

        let expansion_method_regex = Regex::new(r"\s*expansion-method\s+(\S+)\s*").unwrap();
        let input_verilog_regex = Regex::new(r"\s*input-verilog\s+(\S+)\s*").unwrap();
        let output_verilog_regex = Regex::new(r"\s*output-verilog\s+(\S+)\s*").unwrap();
        let top_module_regex = Regex::new(r"\s*top-module\s+(\S+)\s*").unwrap();
        let clock_pins_regex = Regex::new(r"\s*clock-pins\s+(.+)\s*").unwrap();
        let use_primary_io_regex = Regex::new(r"\s*use-primary-io\s+(.+)\s*").unwrap();
        let equivalent_check_regex = Regex::new(r"\s*equivalent-check\s+(.+)\s*").unwrap();
        let ff_definitions_regex = Regex::new(r"\s*ff\s+([^{]+).*").unwrap();
        let inv_definitions_regex = Regex::new(r"\s*inv\s+([^{]+).*").unwrap();
        let empty_line_regex = Regex::new(r"^\s*$").unwrap();

        let mut line_iter = lines.iter().enumerate();
        loop {
            if let Some((i, line)) = line_iter.next() {
                if let Some(cap) = expansion_method_regex.captures(line) {
                    config.expand_method = ExpansionMethods::from_string(&cap.get(1).unwrap().as_str()).unwrap();
                } else if let Some(cap) = input_verilog_regex.captures(line) {
                    config.input_file = cap.get(1).unwrap().as_str().to_string();
                } else if let Some(cap) = output_verilog_regex.captures(line) {
                    config.output_file = cap.get(1).unwrap().as_str().to_string();
                } else if let Some(cap) = top_module_regex.captures(line) {
                    config.top_module = cap.get(1).unwrap().as_str().to_string();
                } else if let Some(cap) = clock_pins_regex.captures(line) {
                    cap.get(1).unwrap().as_str().to_string().split(",").for_each(|pin| {
                        config.clock_pins.push(pin.trim().to_string())
                    });
                } else if let Some(cap) = use_primary_io_regex.captures(line) {
                    config.use_primary_io = !cap.get(1).unwrap().as_str().to_lowercase().eq("no");
                } else if let Some(cap) = equivalent_check_regex.captures(line) {
                    config.equivalent_check = cap.get(1).unwrap().as_str().to_string();
                } else if let Some(_cap) = ff_definitions_regex.captures(line) {
                    // ff define
                    while !line_iter.next().unwrap().1.contains("}") {
                    }
                } else if let Some(_cap) = inv_definitions_regex.captures(line) {
                    // inv define
                    while !line_iter.next().unwrap().1.contains("}") {
                    }
                } else if empty_line_regex.is_match(line) {
                } else {
                    eprintln!("Error: Undefined Option");
                    eprintln!("Syntax error at line {}", i+1);
                    eprintln!("{}", line);
                    panic!("Syntax Error at line {}", i+1);
                }
            } else {
                break;
            }
        }

        Ok(config)

    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExpansionMethods {
    Broadside,
    SkewedLoad,
}

impl ExpansionMethods {
    pub fn from_string(method: &str) -> Option<Self> {
        let method = method.to_lowercase();
        let broadsides = ["broadside", "bs", "loc"];
        let skewedloads = ["skewedload", "sl", "los"];

        if broadsides.iter().any(|s| method.to_lowercase().eq(s)) {
            Some(Broadside)
        } else if skewedloads.iter().any(|s| method.to_lowercase().eq(s)) {
            Some(SkewedLoad)
        } else {
            None
        }
    }
}

impl Default for ExpansionMethods {
    fn default() -> Self {
        Self::Broadside
    }
}

#[cfg(test)]
mod test {
    use std::fs::File;
    use crate::time_expansion::config::ExpansionConfig;
    use crate::time_expansion::config::ExpansionMethods::Broadside;

    #[test]
    fn expansion_config() {
        let ec = ExpansionConfig::from_file("expansion.conf").unwrap();
        assert_eq!(ec.expand_method, Broadside);
        assert_eq!(ec.input_file, "b01_net.v");
        assert_eq!(ec.output_file, "b01_bs_net.v");
        assert_eq!(ec.top_module, "b01");
        assert_eq!(ec.clock_pins, vec!["clock", "reset"]);
        assert_eq!(ec.equivalent_check, "str   NO   FLAG_reg/Q");
        assert!(!ec.use_primary_io);
    }
}
