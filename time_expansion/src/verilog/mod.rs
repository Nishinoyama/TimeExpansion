pub mod ast;
pub mod netlist_serializer;

use crate::time_expansion::config::ExpansionConfig;
use crate::verilog::ast::parser::Parser;
use crate::verilog::ast::token::Lexer;
use crate::verilog::netlist_serializer::NetlistSerializer;
use std::collections::{BTreeSet, HashMap};
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Verilog {
    modules: Vec<Module>,
}

impl Verilog {
    pub fn from_net_list(net_list: String) -> Verilog {
        let lexer = Lexer::from_str(net_list.as_str());
        let tokens = lexer.tokenize();
        let parser = Parser::from_tokens(tokens);
        parser.verilog().unwrap()
    }
    pub fn from_file(file_name: String) -> std::io::Result<Verilog> {
        let verilog_file = File::open(file_name)?;
        let verilog_buf_reader = BufReader::new(verilog_file);
        let mut net_list = String::new();
        for line in verilog_buf_reader.lines() {
            let line = line.unwrap().split("//").next().unwrap().to_string();
            net_list += &line;
            net_list += &String::from("\n");
        }
        Ok(Self::from_net_list(net_list))
    }
    pub fn from_config(config: &ExpansionConfig) -> Self {
        Self::from_file(config.get_input_file().clone()).unwrap()
    }
    pub fn push_module(&mut self, module: Module) {
        self.modules.push(module);
    }
    pub fn get_module(&self, name: &String) -> Option<&Module> {
        self.modules.iter().find(|m| m.name.eq(name))
    }
}

impl NetlistSerializer for Verilog {
    fn gen(&self) -> String {
        format!(
            "{modules}",
            modules = self
                .modules
                .iter()
                .map(|module| module.gen())
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
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
            self.inputs
                .insert(range.clone(), vec![input].into_iter().collect());
        }
    }
    pub fn remove_input(&mut self, input: &String) {
        self.inputs.iter_mut().for_each(|(_, inputs)| {
            inputs.remove(input);
        });
    }
    pub fn push_output(&mut self, range: &SignalRange, output: String) {
        if let Some(outputs) = self.outputs.get_mut(range) {
            outputs.insert(output);
        } else {
            self.outputs
                .insert(range.clone(), vec![output].into_iter().collect());
        }
    }
    pub fn remove_output(&mut self, output: &String) {
        self.outputs.iter_mut().for_each(|(_, outputs)| {
            outputs.remove(output);
        });
    }
    pub fn push_wire(&mut self, range: &SignalRange, wire: String) {
        if let Some(wires) = self.wires.get_mut(range) {
            wires.insert(wire);
        } else {
            self.wires
                .insert(range.clone(), vec![wire].into_iter().collect());
        }
    }
    pub fn push_assign(&mut self, assign: String) {
        self.assigns.push(assign);
    }
    pub fn push_gate(&mut self, ident: String, gate: Gate) {
        self.gates.insert(ident, gate);
    }
    pub fn remove_gate(&mut self, ident: &String) -> Option<Gate> {
        self.gates.remove(ident)
    }
    pub fn get_gates(&self) -> &HashMap<String, Gate> {
        &self.gates
    }
}

impl NetlistSerializer for Module {
    fn gen(&self) -> String {
        let mut module = format!(
            "module {ident} ( {ports} );\n",
            ident = self.name,
            ports = self
                .inputs
                .iter()
                .chain(self.outputs.iter())
                .map(|(_, signals)| Self::multi_gen(signals, ", "),)
                .collect::<Vec<_>>()
                .join(", "),
        );
        for (r, s) in &self.inputs {
            if !s.is_empty() {
                module += &format!(
                    "  input {range}{inputs};\n",
                    range = r.gen(),
                    inputs = Self::multi_gen(s, ", "),
                )
            }
        }
        for (r, s) in &self.outputs {
            if !s.is_empty() {
                module += &format!(
                    "  output {range}{outputs};\n",
                    range = r.gen(),
                    outputs = Self::multi_gen(s, ", "),
                )
            }
        }
        for (r, s) in &self.wires {
            if !s.is_empty() {
                module += &format!(
                    "  wire {range}{wires};\n",
                    range = r.gen(),
                    wires = Self::multi_gen(s, ", "),
                )
            }
        }
        for assign in &self.assigns {
            module += &format!("  assign {};\n", assign);
        }
        module += "\n";
        for (ident, gate) in &self.gates {
            module += &format!(
                "  {gate_name} {ident} {gate};\n",
                gate_name = gate.name,
                ident = ident,
                gate = gate.gen()
            )
        }
        module + "endmodule\n"
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum SignalRange {
    Multiple((String, String)),
    Single,
}

impl NetlistSerializer for SignalRange {
    fn gen(&self) -> String {
        match self {
            Self::Multiple((r, l)) => format!("[{}:{}] ", r, l),
            _ => format!(""),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Gate {
    name: String,
    ports: Vec<(String, String)>,
}

impl Gate {
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }
    pub fn get_name(&self) -> &String {
        &self.name
    }
    pub fn push_port(&mut self, port: String, wire: String) {
        self.ports.push((port, wire));
    }
    pub fn get_ports(&self) -> &Vec<(String, String)> {
        &self.ports
    }
    pub fn get_port_by_name(&self, port_name: &String) -> Option<&(String, String)> {
        self.ports.iter().find(|(port, _)| port.eq(port_name))
    }
}

impl NetlistSerializer for Gate {
    fn gen(&self) -> String {
        format!(
            "( {} )",
            self.ports
                .iter()
                .map(|(port, wire)| format!(".{}({})", port, wire))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

#[cfg(test)]
mod test {
    use crate::verilog::Verilog;

    #[test]
    fn expansion_config() {
        Verilog::from_file("b15_net.v".to_string()).ok();
    }
}
