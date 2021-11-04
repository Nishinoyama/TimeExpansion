use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::option::Option::Some;
use std::process::exit;
use regex::Regex;
use crate::time_expansion::config::ExpansionMethod::{Broadside, SkewedLoad};
use crate::time_expansion::ff_definition::FFDefinition;
use crate::time_expansion::inv_definition::InvDefinition;

#[derive(Clone, Default, Debug)]
pub struct ExpansionConfig {
    expand_method: Option<ExpansionMethod>,
    input_file: String,
    output_file: String,
    top_module: String,

    clock_pins: Vec<String>,

    use_primary_io: bool,
    equivalent_check: String,
    ff_definitions: Vec<FFDefinition>,
    inv_definition: InvDefinition,

}

impl ExpansionConfig {

    fn read_file(&self, file_name: &str ) -> std::io::Result<Vec<String>> {
        let config_file = File::open(file_name)?;
        let mut config_buf_reader = BufReader::new(config_file);
        let mut lines = Vec::new();
        for line in config_buf_reader.lines() {
            let line = line.unwrap().split("#").next().unwrap().to_string();
            lines.push(line);
        }
        Ok(lines)
    }

    fn parse_lines(&mut self, lines: Vec<String>) -> Result<(), String> {
        let expansion_method_regex = Regex::new(r"\s*expansion-method\s+(\S+)\s*").unwrap();
        let input_verilog_regex = Regex::new(r"\s*input-verilog\s+(\S+)\s*").unwrap();
        let output_verilog_regex = Regex::new(r"\s*output-verilog\s+(\S+)\s*").unwrap();
        let top_module_regex = Regex::new(r"\s*top-module\s+(\S+)\s*").unwrap();
        let clock_pins_regex = Regex::new(r"\s*clock-pins\s+(.+)\s*").unwrap();
        let use_primary_io_regex = Regex::new(r"\s*use-primary-io\s+(.+)\s*").unwrap();
        let equivalent_check_regex = Regex::new(r"\s*equivalent-check\s+(.+)\s*").unwrap();
        let ff_definitions_regex = Regex::new(r"\s*ff\s+([^{]+)\s*\{.*").unwrap();
        let inv_definitions_regex = Regex::new(r"\s*inv\s+([^{]+)\s*\{.*").unwrap();
        let empty_line_regex = Regex::new(r"^\s*$").unwrap();

        let mut line_iter = lines.iter().enumerate();
        while let Some((i, line)) = line_iter.next() {
            if let Some(cap) = expansion_method_regex.captures(line) {
                self.expand_method = ExpansionMethod::from_string(&cap.get(1).unwrap().as_str());
            } else if let Some(cap) = input_verilog_regex.captures(line) {
                self.input_file = cap.get(1).unwrap().as_str().to_string();
            } else if let Some(cap) = output_verilog_regex.captures(line) {
                self.output_file = cap.get(1).unwrap().as_str().to_string();
            } else if let Some(cap) = top_module_regex.captures(line) {
                self.top_module = cap.get(1).unwrap().as_str().to_string();
            } else if let Some(cap) = clock_pins_regex.captures(line) {
                cap.get(1).unwrap().as_str().split(",").for_each(|pin| {
                    self.clock_pins.push(pin.trim().to_string())
                });
            } else if let Some(cap) = use_primary_io_regex.captures(line) {
                self.use_primary_io = !cap.get(1).unwrap().as_str().to_lowercase().eq("no");
            } else if let Some(cap) = equivalent_check_regex.captures(line) {
                self.equivalent_check = cap.get(1).unwrap().as_str().to_string();
            } else if let Some(cap) = ff_definitions_regex.captures(line) {
                let mut ff_define = FFDefinition::from_file_iter(&mut line_iter);
                ff_define.set_name(&cap.get(1).unwrap().as_str().trim().to_string());
                self.ff_definitions.push(ff_define);
            } else if let Some(cap) = inv_definitions_regex.captures(line) {
                let mut inv_define = InvDefinition::from_file_iter(&mut line_iter);
                inv_define.set_name(&cap.get(1).unwrap().as_str().trim().to_string());
                self.inv_definition = inv_define;
            } else if empty_line_regex.is_match(line) {
            } else {
                eprintln!("Error: Undefined Option");
                eprintln!("Syntax error at line {}", i+1);
                eprintln!("{}", line);
                return Err(format!("Error: Undefined Option.\nSyntax Error at line {}", i+1));
            }
        }
        Ok(())
    }

    fn verification(&self) -> Result<(), &'static str> {
        if self.clock_pins.is_empty() {
            eprintln!("Warning: clock-pins option is blank. (Asynchronous circuit?)");
        }
        if self.expand_method.is_none() && self.equivalent_check.is_empty() {
            Err("Error: expand-method option must be specified in the configuration file.")
        } else if self.input_file.is_empty() {
            Err("Error: input-file option must be specified in the configuration file.")
        } else if self.output_file.is_empty() {
            Err("Error: output-file option must be specified in the configuration file.")
        } else if self.ff_definitions.is_empty() {
            Err("Error: ff option must be specified at least 1 in the configuration file.")
        } else if self.inv_definition.is_empty() {
            Err(concat!(
                "Error: inv option must be specified in the configuration file\n",
                "       or cannot analyze the following NOT gate type specified in inv option."
            ))
        } else {
            Ok(())
        }
    }

    pub fn from_file( file_name: &str ) -> Result<Self, String> {
        let mut config = Self::default();
        let lines = config.read_file(file_name).unwrap();
        config.parse_lines(lines)?;
        config.verification()?;
        Ok(config)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExpansionMethod {
    Broadside,
    SkewedLoad,
}

impl ExpansionMethod {
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

impl Default for ExpansionMethod {
    fn default() -> Self {
        Self::Broadside
    }
}

#[cfg(test)]
mod test {
    use std::fs::File;
    use crate::time_expansion::config::ExpansionConfig;
    use crate::time_expansion::config::ExpansionMethod::Broadside;
    use crate::time_expansion::ff_definition::FFDefinition;
    use crate::time_expansion::inv_definition::InvDefinition;

    #[test]
    fn expansion_config() {
        let ec = ExpansionConfig::from_file("expansion_example.conf").unwrap();
        println!("{:?}", ec);
        assert_eq!(ec.expand_method, Some(Broadside));
        assert_eq!(ec.input_file, "b01_net.v");
        assert_eq!(ec.output_file, "b01_bs_net.v");
        assert_eq!(ec.top_module, "b01");
        assert_eq!(ec.clock_pins, vec!["clock", "reset"]);
        assert_eq!(ec.equivalent_check, "str   NO   FLAG_reg/Q");
        assert!(!ec.use_primary_io);
        assert_eq!(ec.ff_definitions, vec![
            FFDefinition {
                name: String::from("FD2S"), data_in:vec![String::from("D")],
                data_out: vec![String::from("Q"), String::from("QN")],
                control: vec![String::from("TI"), String::from("TE"),
                              String::from("CP"), String::from("CD")],
            },
            FFDefinition {
                name: String::from("FD2"), data_in:vec![String::from("D")],
                data_out: vec![String::from("Q"), String::from("QN")],
                control: vec![String::from("CP"), String::from("CD")],
            },
            FFDefinition {
                name: String::from("FD1S"), data_in:vec![String::from("D")],
                data_out: vec![String::from("Q"), String::from("QN")],
                control: vec![String::from("TI"), String::from("TE"), String::from("CP")],
            },
            FFDefinition {
                name: String::from("FD1"), data_in:vec![String::from("D")],
                data_out: vec![String::from("Q"), String::from("QN")],
                control: vec![String::from("CP")],
            },
        ]);
        assert_eq!(ec.inv_definition, InvDefinition {
            name: String::from("IV"), input: String::from("A"), output: String::from("Z")
        })
    }
}
