use std::collections::{BTreeSet, HashMap};
use crate::time_expansion::config::ExpansionConfig;
use crate::verilog::ast::parser::Parser;
use crate::verilog::ast::token::Lexer;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

#[derive(Clone, Debug, Default)]
pub struct Verilog {
    modules: Vec<Module>,
}

impl Verilog {
    fn from_file(file_name: String) -> std::io::Result<Verilog> {
        let verilog_file = File::open(file_name)?;
        let verilog_buf_reader = BufReader::new(verilog_file);
        let mut verilog_string = String::new();
        for line in verilog_buf_reader.lines() {
            let line = line.unwrap().split("//").next().unwrap().to_string();
            verilog_string += &line;
            verilog_string += &String::from("\n");
        }
        let lexer = Lexer::from_str(verilog_string.as_str());
        let parser = Parser::from_tokens(lexer.tokenize());
        let verilog = parser.verilog().unwrap();
        Ok(verilog)
    }
    pub fn from_config(config: &ExpansionConfig) -> Self {
        Verilog::from_file(config.get_input_file().clone()).unwrap()
    }
    pub fn push_module(&mut self, module: Module) {
        self.modules.push(module);
    }
}

#[derive(Clone, Debug, Default)]
pub struct Module {
    name: String,
    inputs: HashMap<SignalRange, BTreeSet<String>>,
    outputs: HashMap<SignalRange, BTreeSet<String>>,
    wires: HashMap<SignalRange, BTreeSet<String>>,
    assigns: Vec<String>,
    gates: HashMap<String, Gate>,
}

impl Module {
    pub fn set_name(&mut self, name: String) {
        self.name = name
    }
    pub fn push_input(&mut self, range: &SignalRange, input: String) {
        if let Some(inputs) = self.inputs.get_mut(range) {
            inputs.insert(input);
        } else {
            self.inputs.insert(range.clone(), vec![input].into_iter().collect());
        }
    }
    pub fn push_output(&mut self, range: &SignalRange, output: String) {
        if let Some(outputs) = self.outputs.get_mut(range) {
            outputs.insert(output);
        } else {
            self.outputs.insert(range.clone(), vec![output].into_iter().collect());
        }
    }
    pub fn push_wire(&mut self, range: &SignalRange, wire: String) {
        if let Some(wires) = self.wires.get_mut(range) {
            wires.insert(wire);
        } else {
            self.wires.insert(range.clone(), vec![wire].into_iter().collect());
        }
    }
    pub fn push_assign(&mut self, assign: String) {
        self.assigns.push(assign);
    }
    pub fn push_gate(&mut self, ident: String, gate: Gate) {
        self.gates.insert(ident, gate);
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum SignalRange {
    Multiple((String, String)),
    Single,
}

#[derive(Clone, Debug, Default)]
pub struct Gate {
    name: String,
    ports: Vec<(String, String)>,
}

impl Gate {
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }
    pub fn push_port(&mut self, port: String, wire: String) {
        self.ports.push((port, wire));
    }
}

#[cfg(test)]
mod test {
    use crate::time_expansion::config::ExpansionConfig;
    use crate::verilog::verilog::Verilog;

    #[test]
    fn expansion_config() {
        let ec = ExpansionConfig::from_file("expansion_example.conf").unwrap();
        let verilog = Verilog::from_config(&ec);
        eprintln!("{:?}", verilog);
    }
}
