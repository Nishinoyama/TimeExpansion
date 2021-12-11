mod ast;
pub mod fault;
pub mod netlist_serializer;

use crate::verilog::ast::parser::{ParseError, Parser};
use crate::verilog::ast::token::Lexer;
use crate::verilog::fault::Fault;
use crate::verilog::netlist_serializer::NetlistSerializer;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Verilog {
    modules: Vec<Module>,
}

impl Verilog {
    pub fn from_net_list(net_list: &str) -> Result<Verilog, VerilogError> {
        let lexer = Lexer::from_str(net_list);
        let tokens = lexer.tokenize();
        let parser = Parser::from_tokens(tokens);
        Ok(parser.verilog()?)
    }
    pub fn from_file(file_name: &str) -> Result<Verilog, VerilogError> {
        let verilog_file = File::open(file_name)?;
        let verilog_buf_reader = BufReader::new(verilog_file);
        let mut net_list = String::new();
        for line in verilog_buf_reader.lines() {
            let line = line.unwrap().split("//").next().unwrap().to_string();
            net_list += &line;
            net_list += &String::from("\n");
        }
        Ok(Self::from_net_list(net_list.as_str())?)
    }
    pub fn push_module(&mut self, module: Module) {
        self.modules.push(module);
    }
    pub fn module_by_name(&self, name: &str) -> Option<&Module> {
        self.modules.iter().find(|m| m.name.eq(name))
    }
    pub fn module_by_name_mut(&mut self, name: &str) -> Option<&mut Module> {
        self.modules.iter_mut().find(|m| m.name.eq(name))
    }
    pub fn take_module_buy_name(&mut self, name: &str) -> Option<Module> {
        let md = self.modules.iter().position(|m| m.name.eq(name));
        if let Some(poll_module) = md {
            Some(self.modules.remove(poll_module))
        } else {
            None
        }
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
    inputs: HashSet<Wire>,
    outputs: HashSet<Wire>,
    wires: HashSet<Wire>,
    assigns: Vec<String>,
    gates: BTreeMap<String, Gate>,
}

impl Module {
    pub fn new_with_name(name: String) -> Self {
        let mut m = Module::default();
        *m.name_mut() = name;
        m
    }
    pub fn clone_with_name_prefix(&self, prefix: &str) -> Self {
        let mut clone = self.clone();
        clone.name = format!("{}{}", clone.name, prefix);
        clone
    }
    pub fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }
    pub fn name(&self) -> &String {
        &self.name
    }
    pub fn push_input(&mut self, input: Wire) -> bool {
        self.inputs.insert(input)
    }
    pub fn remove_input(&mut self, input: &Wire) -> bool {
        self.inputs.remove(input)
    }
    pub fn push_output(&mut self, output: Wire) -> bool {
        self.outputs.insert(output)
    }
    pub fn remove_output(&mut self, output: &Wire) -> bool {
        self.outputs.remove(output)
    }
    pub fn push_wire(&mut self, wire: Wire) -> bool {
        self.wires.insert(wire)
    }
    pub fn assigns(&self) -> &Vec<String> {
        &self.assigns
    }
    pub fn push_assign(&mut self, assign: String) {
        self.assigns.push(assign);
    }
    pub fn remove_assign(&mut self, assign: &String) -> Option<String> {
        if let Some(i) = self.assigns.iter().position(|s| s.eq(assign)) {
            Some(self.assigns.remove(i))
        } else {
            None
        }
    }
    pub fn gate_by_name(&self, ident: &str) -> Option<&Gate> {
        self.gates.get(ident)
    }
    pub fn push_gate(&mut self, ident: String, gate: Gate) {
        self.gates.insert(ident, gate);
    }
    pub fn gate_mut_by_name(&mut self, ident: &str) -> Option<&mut Gate> {
        self.gates.get_mut(ident)
    }
    pub fn remove_gate(&mut self, ident: &str) -> Option<Gate> {
        self.gates.remove(ident)
    }
    pub fn inputs(&self) -> &HashSet<Wire> {
        &self.inputs
    }
    pub fn outputs(&self) -> &HashSet<Wire> {
        &self.outputs
    }
    pub fn gates(&self) -> &BTreeMap<String, Gate> {
        &self.gates
    }
    pub fn pins(&self) -> Vec<&Wire> {
        self.inputs.iter().chain(&self.outputs).collect()
    }
    // TODO: does Module have this responsibility?
    pub fn add_observation_point(
        &mut self,
        signal: &str,
        sa_value: bool,
    ) -> Result<String, ModuleError> {
        let signal = signal.split("/").collect::<Vec<_>>();
        let slow_to = if sa_value { "stf" } else { "str" };
        if signal.len() == 1 {
            let primary_io = signal[0];
            let observable_wire = format!("{}_tp_{}", primary_io, slow_to);
            self.push_assign(format!("{} = {}", observable_wire, primary_io));
            self.push_output(Wire::new_single(observable_wire.clone()));
            Ok(observable_wire)
        } else if signal.len() == 2 {
            let gate_name = signal[0];
            let port = signal[1];
            if let Some(gate) = self.gates().get(gate_name).cloned() {
                let port_wire = gate.port_by_name(&port.to_string()).unwrap();
                let wire = port_wire.wire();
                let observable_wire = format!("{}_tp_{}", signal.join("_"), slow_to);
                self.push_assign(format!("{} = {}", observable_wire, wire));
                self.push_output(Wire::new_single(observable_wire.clone()));
                Ok(observable_wire)
            } else {
                Err(ModuleError::UndefinedSignal(format!(
                    "Such a signal named {} doesn't exist.\nPerhaps, it is FF-related signal.",
                    signal.join("/")
                )))
            }
        } else {
            Err(ModuleError::ExceededStuckAtFaultInsertionDepth(format!(
                "Too depth to observe \"{}\".",
                signal.join("/")
            )))
        }
    }
    // TODO: does Module have this responsibility?
    pub fn insert_stuck_at_fault(
        &self,
        new_module_name: String,
        fault: &Fault,
    ) -> Result<Self, ModuleError> {
        let mut faulty_module = self.clone();
        faulty_module.name = new_module_name;
        let sa_value = format!("1'b{}", if fault.sa_value() { 1 } else { 0 });
        let stuck_signal = fault.location().split("/").collect::<Vec<_>>();
        if stuck_signal.len() == 1 {
            // top level port stuck fault
            let stuck_wire = stuck_signal[0].to_string();
            let stuck_gates = faulty_module
                .gates()
                .clone()
                .into_iter()
                .filter(|(_, gate)| {
                    gate.ports()
                        .iter()
                        .any(|port_wire| stuck_wire.eq(port_wire.wire()))
                });
            for (ident, mut gate) in stuck_gates {
                for port_wire in gate.ports_mut() {
                    let port = port_wire.port().to_string();
                    let wire = port_wire.wire_mut();
                    if stuck_wire.eq(wire) {
                        // TODO: Remove "Z" or "Y" Magic which means output port!
                        if port.contains("Z") || port.contains("Y") || port.contains("Q") {
                            let opened_wire = format!("{}_drained", wire);
                            faulty_module.push_wire(Wire::new_single(opened_wire.clone()));
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
            let port_wire = stuck_gate.port_by_name_mut(&stuck_port_name).unwrap();
            let wire = port_wire.wire_mut();
            // TODO: Remove "Z" or "Y" Magic which means output port!
            if stuck_port_name.contains("Z")
                || stuck_port_name.contains("Y")
                || stuck_port_name.contains("Q")
            {
                let opened_wire = format!("{}_drained", wire);
                faulty_module.push_wire(Wire::new_single(opened_wire.clone()));
                faulty_module.push_assign(format!("{} = {}", wire, sa_value));
                *wire = opened_wire.clone();
            } else {
                *wire = sa_value.clone();
            }
            faulty_module.remove_gate(&stuck_gate_ident);
            faulty_module.push_gate(stuck_gate_ident, stuck_gate)
        } else {
            // too deep level not to insert stuck fault
            return Err(ModuleError::ExceededStuckAtFaultInsertionDepth(format!(
                "Specified fault \"{}\" is too deep to be inserted!",
                stuck_signal.join("/")
            )));
        }
        Ok(faulty_module)
    }
    pub fn to_gate(&self) -> Gate {
        let mut gate = Gate::default();
        *gate.name_mut() = self.name().clone();
        for pin in self.pins() {
            gate.push_port(PortWire::Wire(
                pin.name().to_string(),
                pin.name().to_string(),
            ))
        }
        gate
    }
    fn wires_by_signal_range(wires: &HashSet<Wire>) -> HashMap<SignalRange, Vec<String>> {
        let mut signal_range_wires: HashMap<SignalRange, Vec<String>> = HashMap::new();
        for wire in wires {
            let ident = wire.name();
            let range = wire.range();
            if let Some(w) = signal_range_wires.get_mut(range) {
                w.push(ident.to_string());
            } else {
                signal_range_wires.insert(range.clone(), vec![ident.to_string()]);
            }
        }
        signal_range_wires
    }
}

#[derive(Debug)]
pub enum ModuleError {
    UndefinedSignal(String),
    ExceededStuckAtFaultInsertionDepth(String),
}

impl NetlistSerializer for Module {
    fn gen(&self) -> String {
        let mut module = format!(
            "module {ident} ( {pins} );\n",
            ident = self.name,
            pins = self
                .pins()
                .into_iter()
                .map(|pin| pin.name.clone())
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

impl PartialOrd<Self> for SignalRange {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SignalRange {
    fn cmp(&self, other: &Self) -> Ordering {
        use Ordering::*;
        use SignalRange::*;
        match self {
            Single => match other {
                Single => Equal,
                Multiple(_) => Less,
            },
            Multiple(lrange) => match other {
                Single => Equal,
                Multiple(rrange) => lrange.cmp(rrange),
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Wire {
    range: SignalRange,
    name: String,
}

impl Wire {
    pub fn new(range: SignalRange, name: String) -> Self {
        Wire { range, name }
    }
    pub fn new_single(name: String) -> Self {
        Self::new(SignalRange::Single, name)
    }
    pub fn new_multiple(name: String, left: String, right: String) -> Self {
        Self::new(SignalRange::Multiple((left, right)), name)
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn range(&self) -> &SignalRange {
        &self.range
    }
    pub fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Gate {
    name: String,
    ports: Vec<PortWire>,
}

impl Gate {
    pub fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn push_port(&mut self, port_wire: PortWire) {
        self.ports.push(port_wire);
    }
    pub fn ports(&self) -> &Vec<PortWire> {
        &self.ports
    }
    pub fn ports_mut(&mut self) -> &mut Vec<PortWire> {
        &mut self.ports
    }
    pub fn port_by_name(&self, port_name: &str) -> Option<&PortWire> {
        self.ports.iter().find(|pw| pw.port_is(port_name))
    }
    pub fn port_by_name_mut(&mut self, port_name: &str) -> Option<&mut PortWire> {
        self.ports.iter_mut().find(|pw| pw.port_is(port_name))
    }
    pub fn take_port_by_name(&mut self, port_name: &str) -> Option<PortWire> {
        if let Some(i) = self.ports.iter().position(|r| r.port_is(port_name)) {
            Some(self.ports.remove(i))
        } else {
            None
        }
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
    pub fn wire(&self) -> &str {
        match self {
            Self::Wire(_, wire) => wire,
            Self::Constant(_, wire) => wire,
        }
    }
    pub fn wire_mut(&mut self) -> &mut String {
        match self {
            Self::Wire(_, wire) => wire,
            Self::Constant(_, wire) => wire,
        }
    }
    pub fn port(&self) -> &str {
        match self {
            Self::Wire(port, _) => port,
            Self::Constant(port, _) => port,
        }
    }
    pub fn port_is(&self, port_name: &str) -> bool {
        match self {
            Self::Wire(port, _) => port.eq(port_name),
            Self::Constant(port, _) => port.eq(port_name),
        }
    }
}

impl NetlistSerializer for PortWire {
    fn gen(&self) -> String {
        format!(".{}({})", self.port(), self.wire())
    }
}

#[derive(Debug)]
pub enum VerilogError {
    ParserError(ParseError),
    ModuleError(ModuleError),
    IOError(std::io::Error),
}

impl From<ParseError> for VerilogError {
    fn from(error: ParseError) -> Self {
        Self::ParserError(error)
    }
}

impl From<ModuleError> for VerilogError {
    fn from(error: ModuleError) -> Self {
        Self::ModuleError(error)
    }
}

impl From<std::io::Error> for VerilogError {
    fn from(error: std::io::Error) -> Self {
        Self::IOError(error)
    }
}

#[cfg(test)]
#[allow(unused_variables)]
mod test {
    use crate::verilog::fault::Fault;
    use crate::verilog::netlist_serializer::NetlistSerializer;
    use crate::verilog::{Verilog, VerilogError};

    type VerilogTestResult = Result<(), VerilogError>;

    #[test]
    fn expansion_config() -> VerilogTestResult {
        let verilog = Verilog::from_file("b02_net.v")?;
        Ok(())
    }

    #[test]
    fn insert_fault() -> VerilogTestResult {
        let verilog = Verilog::from_file("b02_net.v")?;
        let module = verilog.modules.get(0).unwrap();
        eprintln!("{}", module.gen());
        let fmodule = module.insert_stuck_at_fault(
            String::from("b02_ft"),
            &Fault::new(String::from("U19/A"), false),
        )?;
        eprintln!("{}", fmodule.gen());
        let fmodule = module.insert_stuck_at_fault(
            String::from("b02_ft"),
            &Fault::new(String::from("U19/Z"), false),
        )?;
        eprintln!("{}", fmodule.gen());
        let fmodule = module.insert_stuck_at_fault(
            String::from("b02_ft"),
            &Fault::new(String::from("linea"), false),
        )?;
        eprintln!("{}", fmodule.gen());
        let fmodule = module.insert_stuck_at_fault(
            String::from("b02_ft"),
            &Fault::new(String::from("u"), false),
        )?;
        eprintln!("{}", fmodule.gen());
        Ok(())
    }

    #[test]
    fn add_observation_point() -> VerilogTestResult {
        let mut verilog = Verilog::from_file("b02_net.v")?;
        let mut module = verilog.take_module_buy_name(&String::from("b02")).unwrap();
        eprintln!("{}", module.gen());
        module.add_observation_point(&String::from("U24/A"), false)?;
        eprintln!("{}", module.gen());
        module.add_observation_point(&String::from("u"), true)?;
        eprintln!("{}", module.gen());
        Ok(())
    }
}
