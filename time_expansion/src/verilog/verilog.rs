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
        println!("{:?}", parser);
        let verilog = parser.verilog().unwrap();
        Ok(verilog)
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
    pub fn push_module(&mut self, module: Module) {
        self.modules.push(module);
    }
}

#[derive(Clone, Debug, Default)]
pub struct Module {
    name: String,
    inputs: Vec<Signal>,
    outputs: Vec<Signal>,
    wires: Vec<Signal>,
    assigns: Vec<String>,
    flipflop_definitions: Vec<String>,
    combination_circuits: Vec<String>,
}

impl Module {
    pub fn set_name(&mut self, name: String) {
        self.name = name
    }
    pub fn push_input(&mut self, input: Signal) {
        self.inputs.push(input);
    }
    pub fn push_output(&mut self, output: Signal) {
        self.outputs.push(output);
    }
    pub fn push_wire(&mut self, wire: Signal) {
        self.wires.push(wire);
    }
    pub fn push_assign(&mut self, assign: String) {
        self.assigns.push(assign);
    }
}

#[derive(Clone, Debug)]
pub enum Signal {
    Multiple((String, String), String),
    Single(String),
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
