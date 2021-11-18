pub mod ast;
pub mod netlist_serializer;

use crate::time_expansion::config::ExpansionConfig;
use crate::verilog::ast::parser::Parser;
use crate::verilog::ast::token::Lexer;
use crate::verilog::netlist_serializer::NetlistSerializer;
use std::collections::HashMap;
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
    inputs: HashMap<String, SignalRange>,
    outputs: HashMap<String, SignalRange>,
    wires: HashMap<String, SignalRange>,
    assigns: Vec<String>,
    gates: HashMap<String, Gate>,
}

impl Module {
    pub fn set_name(&mut self, name: String) {
        self.name = name
    }
    pub fn push_input(&mut self, range: SignalRange, input: String) {
        self.inputs.insert(input, range);
    }
    pub fn remove_input(&mut self, input: &String) {
        self.inputs.remove(input);
    }
    pub fn push_output(&mut self, range: SignalRange, output: String) {
        self.outputs.insert(output, range);
    }
    pub fn remove_output(&mut self, output: &String) {
        self.outputs.remove(output);
    }
    pub fn push_wire(&mut self, range: SignalRange, wire: String) {
        self.wires.insert(wire, range);
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
    pub fn get_ports(&self) -> Vec<(&String, &SignalRange)> {
        self.inputs.iter().chain(&self.outputs).collect()
    }
    fn wires_by_signal_range(
        wires: &HashMap<String, SignalRange>,
    ) -> HashMap<SignalRange, Vec<String>> {
        let mut signal_range_wires: HashMap<SignalRange, Vec<String>> = HashMap::new();
        for (ident, range) in wires {
            if let Some(w) = signal_range_wires.get_mut(range) {
                w.push(ident.clone());
            } else {
                signal_range_wires.insert(range.clone(), vec![ident.clone()]);
            }
        }
        signal_range_wires
    }
    pub fn insert_stuck_at_fault(
        &self,
        new_module_name: String,
        stuck_signal: &String,
        sa_value: usize,
    ) -> Self {
        let mut faulty_module = self.clone();
        faulty_module.name = new_module_name;
        let sa_value = format!("1'b{}", sa_value);
        let stuck_signal = stuck_signal.split("/").collect::<Vec<_>>();
        if stuck_signal.len() == 1 {
            // top level port stuck fault
            let stuck_wire = stuck_signal[0].to_string();
            let stuck_gates = faulty_module
                .get_gates()
                .clone()
                .into_iter()
                .filter(|(_, gate)| {
                    gate.get_ports()
                        .iter()
                        .any(|port_wire| stuck_wire.eq(port_wire.get_wire()))
                });
            for (ident, mut gate) in stuck_gates {
                for port_wire in gate.get_ports_mut() {
                    let port = port_wire.get_port().clone();
                    let wire = port_wire.get_wire_mut();
                    if stuck_wire.eq(wire) {
                        // TODO: Remove "Z" or "Y" Magic which means output port!
                        if port.contains("Z") || port.contains("Y") || port.contains("Q") {
                            let opened_wire = format!("{}_drained", wire);
                            faulty_module.push_wire(SignalRange::Single, opened_wire.clone());
                            faulty_module.push_assign(format!("{} = {}", wire, sa_value));
                            *wire = opened_wire.clone();
                        } else {
                            *wire = sa_value.clone();
                        }
                    }
                }
                faulty_module.remove_gate(&ident);
                faulty_module.push_gate(ident, gate)
            }
        } else if stuck_signal.len() == 2 {
            // lower level gate port stuck fault
            let stuck_gate_ident = stuck_signal[0].to_string();
            let stuck_port_name = stuck_signal[1].to_string();
            let mut stuck_gate = faulty_module.gates.get(&stuck_gate_ident).unwrap().clone();
            let port_wire = stuck_gate.get_port_by_name_mut(&stuck_port_name).unwrap();
            let wire = port_wire.get_wire_mut();
            // TODO: Remove "Z" or "Y" Magic which means output port!
            if stuck_port_name.contains("Z")
                || stuck_port_name.contains("Y")
                || stuck_port_name.contains("Q")
            {
                let opened_wire = format!("{}_drained", wire);
                faulty_module.push_wire(SignalRange::Single, opened_wire.clone());
                faulty_module.push_assign(format!("{} = {}", wire, sa_value));
                *wire = opened_wire.clone();
            } else {
                *wire = sa_value.clone();
            }
            faulty_module.remove_gate(&stuck_gate_ident);
            faulty_module.push_gate(stuck_gate_ident, stuck_gate)
        } else {
            // too deep level not to insert stuck fault
            panic!(
                "Specified fault \"{}\" is too deep to be inserted!",
                stuck_signal.join("/")
            )
        }
        faulty_module
    }
}

impl NetlistSerializer for Module {
    fn gen(&self) -> String {
        let mut module = format!(
            "module {ident} ( {ports} );\n",
            ident = self.name,
            ports = self
                .get_ports()
                .into_iter()
                .map(|(ident, _)| ident.clone())
                .collect::<Vec<_>>()
                .join(", "),
        );
        let wires = vec![
            (&self.inputs, "input"),
            (&self.outputs, "output"),
            (&self.wires, "wire"),
        ];
        for (wires, wire_type) in wires {
            for (r, s) in Self::wires_by_signal_range(&wires) {
                module += &format!(
                    "  {wire_type} {range}{wires};\n",
                    wire_type = wire_type,
                    range = r.gen(),
                    wires = Self::multi_gen(&s, ", "),
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
    ports: Vec<PortWire>,
}

impl Gate {
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }
    pub fn get_name(&self) -> &String {
        &self.name
    }
    pub fn push_port(&mut self, port_wire: PortWire) {
        self.ports.push(port_wire);
    }
    pub fn get_ports(&self) -> &Vec<PortWire> {
        &self.ports
    }
    pub fn get_ports_mut(&mut self) -> &mut Vec<PortWire> {
        &mut self.ports
    }
    pub fn get_port_by_name(&self, port_name: &String) -> Option<&PortWire> {
        self.ports.iter().find(|pw| pw.port_is(port_name))
    }
    pub fn get_port_by_name_mut(&mut self, port_name: &String) -> Option<&mut PortWire> {
        self.ports.iter_mut().find(|pw| pw.port_is(port_name))
    }
}

impl NetlistSerializer for Gate {
    fn gen(&self) -> String {
        format!("( {} )", Self::multi_gen(&self.ports, ", "))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PortWire {
    Constant(String, String),
    Wire(String, String),
}

impl PortWire {
    pub fn get_wire(&self) -> &String {
        match self {
            Self::Wire(_, wire) => wire,
            Self::Constant(_, wire) => wire,
        }
    }
    pub fn get_wire_mut(&mut self) -> &mut String {
        match self {
            Self::Wire(_, wire) => wire,
            Self::Constant(_, wire) => wire,
        }
    }
    pub fn get_port(&self) -> &String {
        match self {
            Self::Wire(port, _) => port,
            Self::Constant(port, _) => port,
        }
    }
    pub fn port_is(&self, port_name: &String) -> bool {
        match self {
            Self::Wire(port, _) => port.eq(port_name),
            Self::Constant(port, _) => port.eq(port_name),
        }
    }
}

impl NetlistSerializer for PortWire {
    fn gen(&self) -> String {
        format!(".{}({})", self.get_port(), self.get_wire())
    }
}

#[cfg(test)]
mod test {
    use crate::verilog::netlist_serializer::NetlistSerializer;
    use crate::verilog::Verilog;

    #[test]
    fn expansion_config() {
        let verilog = Verilog::from_file(String::from("b15_net.v")).ok().unwrap();
    }

    #[test]
    fn insert_fault() {
        let verilog = Verilog::from_file(String::from("b02_net.v")).ok().unwrap();
        let module = verilog.modules.get(0).unwrap();
        eprintln!("{}", module.gen());
        let fmodule =
            module.insert_stuck_at_fault(String::from("b02_ft"), &String::from("U19/A"), 0);
        eprintln!("{}", fmodule.gen());
        let fmodule =
            module.insert_stuck_at_fault(String::from("b02_ft"), &String::from("U19/Z"), 0);
        eprintln!("{}", fmodule.gen());
        let fmodule =
            module.insert_stuck_at_fault(String::from("b02_ft"), &String::from("linea"), 0);
        eprintln!("{}", fmodule.gen());
        let fmodule = module.insert_stuck_at_fault(String::from("b02_ft"), &String::from("u"), 0);
        eprintln!("{}", fmodule.gen());
    }
}
