use crate::time_expansion::config::ExpansionMethod::{Broadside, SkewedLoad};
use crate::verilog::netlist_serializer::NetlistSerializer;
use crate::verilog::{Gate, Module, PortWire, SignalRange, Verilog};
use regex::Regex;
use std::fmt::format;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::iter::Enumerate;
use std::option::Option::Some;
use std::slice::Iter;

#[derive(Clone, Default, Debug)]
pub struct ExpansionConfig {
    expand_method: Option<ExpansionMethod>,
    input_file: String,
    output_file: String,
    top_module: String,

    clock_pins: Vec<String>,

    use_primary_io: bool,
    equivalent_check: (bool, String),
    ff_definitions: Vec<FFDefinition>,
    inv_definition: InvDefinition,
}

impl ExpansionConfig {
    fn read_file(&self, file_name: &str) -> std::io::Result<Vec<String>> {
        let config_file = File::open(file_name)?;
        let config_buf_reader = BufReader::new(config_file);
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
                cap.get(1)
                    .unwrap()
                    .as_str()
                    .split(",")
                    .for_each(|pin| self.clock_pins.push(pin.trim().to_string()));
            } else if let Some(cap) = use_primary_io_regex.captures(line) {
                self.use_primary_io = !cap.get(1).unwrap().as_str().to_lowercase().eq("no");
            } else if let Some(cap) = equivalent_check_regex.captures(line) {
                let ec_regex = Regex::new(r"\s*(st[rf])\s+(\S+)\s+(\S+).*").unwrap();
                if let Some(ec_cap) = ec_regex.captures(cap.get(1).unwrap().as_str()) {
                    self.equivalent_check = (
                        ec_cap.get(1).unwrap().as_str().eq("stf"),
                        ec_cap.get(3).unwrap().as_str().to_string(),
                    )
                } else {
                    return Err(format!(
                        "Error: Equivalent check fault syntax Error at line {}",
                        i + 1
                    ));
                }
            } else if let Some(cap) = ff_definitions_regex.captures(line) {
                let mut ff_define = FFDefinition::from_file_iter(&mut line_iter);
                *ff_define.get_name_mut() = cap.get(1).unwrap().as_str().trim().to_string();
                self.ff_definitions.push(ff_define);
            } else if let Some(cap) = inv_definitions_regex.captures(line) {
                let mut inv_define = InvDefinition::from_file_iter(&mut line_iter);
                *inv_define.get_name_mut() = cap.get(1).unwrap().as_str().trim().to_string();
                self.inv_definition = inv_define;
            } else if empty_line_regex.is_match(line) {
            } else {
                eprintln!("Error: Undefined Option");
                eprintln!("Syntax error at line {}", i + 1);
                eprintln!("{}", line);
                return Err(format!(
                    "Error: Undefined Option.\nSyntax Error at line {}",
                    i + 1
                ));
            }
        }
        Ok(())
    }
    fn verification(&self) -> Result<(), &'static str> {
        if self.clock_pins.is_empty() {
            eprintln!("Warning: clock-pins option is blank. (Asynchronous circuit?)");
        }
        if self.expand_method.is_none() && self.equivalent_check.1.is_empty() {
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
    /// Generate config from file specifying config.
    /// [`Err`]\([`String`]) if file cannot compiled or is wrong.
    pub fn from_file(file_name: &str) -> Result<Self, String> {
        let mut config = Self::default();
        let lines = config.read_file(file_name).unwrap();
        config.parse_lines(lines)?;
        config.verification()?;
        Ok(config)
    }
    /// Returns input file name specified by config.
    pub fn get_input_file(&self) -> &String {
        &self.input_file
    }
    fn extract_ff_gates(&self, module: &Module) -> Vec<(&FFDefinition, String, Gate)> {
        module
            .get_gates()
            .iter()
            .filter_map(|(s, g)| {
                if let Some(ff_type) = self
                    .ff_definitions
                    .iter()
                    .find(|ff_def| g.get_name().eq(&ff_def.name))
                {
                    Some((ff_type, s.clone(), g.clone()))
                } else {
                    None
                }
            })
            .collect()
    }
    /// Generates combinational part from full scan designed circuitry module, their Pseudo Inputs' and Pseudo Outputs' port name.
    /// FFs', in the module, inputs and outputs become the extracted combinational module's inputs and outputs respectively.
    /// Such inputs and outputs are called Pseudo Inputs/Outputs
    pub fn extract_combinational_part(
        &self,
        module: &Module,
    ) -> (Module, Vec<String>, Vec<String>) {
        let mut combinational_part = module.clone();
        let mut pseudo_primary_inputs = Vec::new();
        let mut pseudo_primary_outputs = Vec::new();
        self.extract_ff_gates(&module)
            .into_iter()
            .enumerate()
            .for_each(|(i, (ff_def, ident, ff_gate))| {
                let ppi = format!("ppi_{}_{}", i + 1, ident);
                let ppo = format!("ppo_{}_{}", i + 1, ident);
                for port in ff_def.data_in.iter() {
                    if let Some(port_wire) = ff_gate.get_port_by_name(port) {
                        combinational_part.push_assign(format!(
                            "{ppo} = {wire_from_port}",
                            ppo = ppo,
                            wire_from_port = port_wire.get_wire(),
                        ));
                        combinational_part.push_output(SignalRange::Single, ppo.clone());
                    }
                }
                for port in ff_def.data_out.iter() {
                    if let Some(port_wire) = ff_gate.get_port_by_name(port) {
                        combinational_part.push_input(SignalRange::Single, ppi.clone());
                        if port.contains("N") {
                            combinational_part.push_gate(
                                format!("UN{}", i + 1),
                                self.inv_definition.to_gate(&ppi, port_wire.get_wire()),
                            );
                        } else {
                            combinational_part.push_assign(format!(
                                "{} = {}",
                                port_wire.get_wire(),
                                ppi
                            ));
                        }
                    }
                }
                combinational_part.remove_gate(&ident);

                pseudo_primary_inputs.push(ppi);
                pseudo_primary_outputs.push(ppo);
            });
        for clock in self.clock_pins.iter() {
            combinational_part.remove_input(clock)
        }
        for (test_s_pin, _) in module
            .ports()
            .into_iter()
            .filter(|(pin, _)| pin.contains("test_s"))
        {
            combinational_part.remove_input(test_s_pin);
            combinational_part.remove_output(test_s_pin);
        }
        (
            combinational_part,
            pseudo_primary_inputs,
            pseudo_primary_outputs,
        )
    }
    /// Generates time expansion model from full scan designed circuitry module.
    /// Expansion method is preferred by [`ExpansionMethod`]
    /// if not `use_primary_io`, primary inputs will be restricted and primary outputs will be masked.
    pub fn time_expand(&self) -> Verilog {
        match self.expand_method {
            Some(Broadside) => {
                let verilog = Verilog::from_config(self);
                let top_module = verilog.get_module(&self.top_module).unwrap();
                let (combinational_part, ppis, ppos) = self.extract_combinational_part(top_module);
                let clone_circuit_c1 = combinational_part.clone_with_name_prefix("_cmb_c1");
                let clone_circuit_c2 = combinational_part.clone_with_name_prefix("_cmb_c2");
                let mut gate_c1 = clone_circuit_c1.to_gate();
                let mut gate_c2 = clone_circuit_c2.to_gate();
                let mut expanded_module = Module::new_with_name(format!("{}_bs", self.top_module));

                // connect bs_model inputs to c1 pi, ppis
                for (input, range) in clone_circuit_c1.get_inputs().iter() {
                    *gate_c1.get_port_by_name_mut(&input).unwrap().get_wire_mut() =
                        format!("{}_c1", input);
                    expanded_module.push_input(range.clone(), format!("{}_c1", input));
                }

                // set c1 ppos, remove pos,
                for (output, range) in clone_circuit_c1.get_outputs().iter() {
                    *gate_c1
                        .get_port_by_name_mut(&output)
                        .unwrap()
                        .get_wire_mut() = format!("{}_c1", output);
                    if ppos.iter().any(|ppo| output.contains(ppo)) {
                        expanded_module.push_wire(range.clone(), format!("{}_c1", output));
                    } else {
                        gate_c1.remove_port_by_name(output);
                    }
                }

                // chain c1 ppi to c2 ppo
                for (ppi, ppo) in ppis.iter().zip(ppos.iter()) {
                    expanded_module.push_assign(format!("{}_c2 = {}_c1", ppi, ppo));
                }

                // chain c1 pi wires (bs_model inputs if use_primary_io) to c2 ppi
                for (input, range) in clone_circuit_c1.get_inputs().iter() {
                    *gate_c2.get_port_by_name_mut(&input).unwrap().get_wire_mut() =
                        format!("{}_c2", input);
                    if !ppis.iter().any(|ppi| input.contains(ppi)) {
                        if self.use_primary_io {
                            expanded_module.push_input(range.clone(), format!("{}_c2", input));
                        } else {
                            expanded_module.push_wire(range.clone(), format!("{}_c2", input));
                            expanded_module.push_assign(format!("{}_c2 = {}_c1", input, input));
                        }
                    }
                }

                // chain c2 ppos (and pos if use_primary_io) to bs_model outputs
                // remove c2 pos if not use_primary_io
                for (output, range) in clone_circuit_c1.get_outputs().iter() {
                    *gate_c2
                        .get_port_by_name_mut(&output)
                        .unwrap()
                        .get_wire_mut() = format!("{}_c2", output);
                    if self.use_primary_io || ppos.iter().any(|ppo| output.contains(ppo)) {
                        expanded_module.push_wire(range.clone(), format!("{}_c2", output));
                        expanded_module.push_output(range.clone(), output.clone());
                        expanded_module.push_assign(format!("{} = {}_c2", output, output));
                    } else {
                        gate_c2.remove_port_by_name(output);
                    }
                }

                expanded_module.push_gate(String::from("c1"), gate_c1);
                expanded_module.push_gate(String::from("c2"), gate_c2);

                let mut verilog = Verilog::default();
                verilog.push_module(expanded_module);
                verilog.push_module(clone_circuit_c1);
                verilog.push_module(clone_circuit_c2);

                verilog
            }
            Some(SkewedLoad) => Verilog::default(),
            _ => Verilog::default(),
        }
    }
    ///
    /// insert restricted value gates for generating atpg model
    ///
    pub fn atpg_model_time_expand(&self) -> Result<Verilog, String> {
        let mut atpg_model = self.time_expand();
        let c1_name = format!("{}_cmb_c1", self.top_module);
        let c2_name = format!("{}_cmb_c2", self.top_module);
        let te_module_name = format!("{}_bs", self.top_module);

        let c2_outputs = atpg_model
            .get_module(&c2_name)
            .unwrap()
            .get_outputs()
            .clone();

        // gen observable wire in c1 for restriction
        let c1_module = atpg_model.get_module_mut(&c1_name).unwrap();
        let observable_wire = c1_module.add_observation_point(&self.equivalent_check.1)?;

        // take restriction wire from c1
        let te_module = atpg_model.get_module_mut(&te_module_name).unwrap();
        let restriction_wire = observable_wire;
        te_module.push_wire(SignalRange::Single, restriction_wire.clone());
        let mut c1_gate = te_module.gate_mut_by_name(&String::from("c1")).unwrap();
        c1_gate.push_port(PortWire::Wire(
            restriction_wire.clone(),
            restriction_wire.clone(),
        ));

        let restricted_outputs = c2_outputs
            .keys()
            .into_iter()
            .filter_map(|ppo| {
                te_module
                    .assigns()
                    .iter()
                    .find(|assign| assign.ends_with(&format!("{}_c2", ppo)))
            })
            .cloned()
            .collect::<Vec<_>>();

        restricted_outputs
            .into_iter()
            .enumerate()
            .for_each(|(i, res_assign)| {
                use crate::verilog::PortWire::Wire;
                let mut res_out = res_assign.split("=").map(|s| s.trim().to_string());
                let po = res_out.next().unwrap();
                let ppo_c2 = res_out.next().unwrap();
                let ppo_r = format!("{}_{}", ppo_c2, self.equivalent_check.1.replace("/", "_"));
                let mut restriction_gate = Gate::default();
                *restriction_gate.get_name_mut() = String::from(if self.equivalent_check.0 {
                    "AN2"
                } else {
                    "OR2"
                });
                restriction_gate.push_port(Wire(String::from('A'), restriction_wire.clone()));
                restriction_gate.push_port(Wire(String::from('B'), ppo_r.clone()));
                restriction_gate.push_port(Wire(String::from('Z'), po.clone()));
                te_module.push_gate(
                    format!("R{}_{}", i + 1, self.equivalent_check.1.replace("/", "_")),
                    restriction_gate,
                );
                te_module.push_assign(format!("{} = {}", ppo_r, ppo_c2));
                te_module.push_wire(SignalRange::Single, ppo_r);
                te_module.remove_assigns_by_assign(&res_assign);
            });

        Ok(atpg_model)
    }
    /// Gen 2 modules for Equivalent-Check at transition fault.
    pub fn equivalent_check(&self) -> Result<Verilog, String> {
        let mut ec_verilog = self.atpg_model_time_expand()?;
        let bs_top_name = format!("{}_bs", self.top_module);

        // build bs_ref and bs_imp
        // replace bs_imp's c2 gate with c2_imp
        let mut bs_ref = ec_verilog.get_module_mut(&bs_top_name).unwrap();
        *bs_ref.get_name_mut() = format!("{}_ref", bs_top_name);
        let mut bs_imp = bs_ref.clone();
        *bs_imp.get_name_mut() = format!("{}_imp", bs_top_name);
        *bs_imp
            .gate_mut_by_name(&String::from("c2"))
            .unwrap()
            .get_name_mut() = format!("{}_cmb_c2_imp", self.top_module);
        ec_verilog.push_module(bs_imp);

        // insert stuck-at-fault into c2
        let mut c2_ref = ec_verilog
            .get_module_mut(&format!("{}_cmb_c2", self.top_module))
            .unwrap();
        let mut c2_imp = c2_ref.insert_stuck_at_fault(
            format!("{}_cmb_c2_imp", self.top_module),
            &self.equivalent_check.1,
            self.equivalent_check.0,
        );
        ec_verilog.push_module(c2_imp);

        Ok(ec_verilog)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExpansionMethod {
    Broadside,
    SkewedLoad,
}

impl ExpansionMethod {
    /// Returns time expansion model by their name.
    /// If name doesn't match any models, return [`None`]
    ///
    /// + Broadside
    ///     + broadside
    ///     + bs
    ///     + loc
    /// + Skewedload
    ///     + skewedload
    ///     + sl
    ///     + los
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

#[derive(Clone, Default, Debug, Eq, PartialEq)]
pub struct FFDefinition {
    name: String,
    data_in: Vec<String>,
    data_out: Vec<String>,
    control: Vec<String>,
}

impl FFDefinition {
    pub fn from_file_iter(line_iter: &mut Enumerate<Iter<String>>) -> Self {
        let mut ff_defines = Self::default();
        let data_in_regex = Regex::new(r"\s*data-in\s+(.+)\s*").unwrap();
        let data_out_regex = Regex::new(r"\s*data-out\s+(.+)\s*").unwrap();
        let control_regex = Regex::new(r"\s*control\s+(.+)\s*").unwrap();
        let empty_line_regex = Regex::new(r"^\s*$").unwrap();

        while let Some((i, ff_line)) = line_iter.next() {
            if ff_line.contains("}") {
                break;
            }
            if let Some(cap) = data_in_regex.captures(ff_line) {
                cap.get(1)
                    .unwrap()
                    .as_str()
                    .split(",")
                    .for_each(|data| ff_defines.data_in.push(data.trim().to_string()));
            } else if let Some(cap) = data_out_regex.captures(ff_line) {
                cap.get(1)
                    .unwrap()
                    .as_str()
                    .split(",")
                    .for_each(|data| ff_defines.data_out.push(data.trim().to_string()));
            } else if let Some(cap) = control_regex.captures(ff_line) {
                cap.get(1)
                    .unwrap()
                    .as_str()
                    .split(",")
                    .for_each(|data| ff_defines.control.push(data.trim().to_string()));
            } else if empty_line_regex.is_match(ff_line) {
            } else {
                eprintln!("Error: Undefined FF Option");
                eprintln!("Syntax error at line {}", i + 1);
                eprintln!("{}", ff_line);
                panic!("Syntax Error at line {}", i + 1);
            }
        }
        return ff_defines;
    }

    pub fn get_name_mut(&mut self) -> &mut String {
        &mut self.name
    }
}

#[derive(Clone, Default, Debug, Eq, PartialEq)]
pub struct InvDefinition {
    name: String,
    input: String,
    output: String,
}

impl InvDefinition {
    pub fn from_file_iter(line_iter: &mut Enumerate<Iter<String>>) -> Self {
        let mut inv_defines = Self::default();
        let input_regex = Regex::new(r"\s*input\s+(\w+)\s*").unwrap();
        let output_regex = Regex::new(r"\s*output\s+(\w+)\s*").unwrap();
        let empty_line_regex = Regex::new(r"^\s*$").unwrap();

        while let Some((i, inv_line)) = line_iter.next() {
            if inv_line.contains("}") {
                break;
            }
            if let Some(cap) = input_regex.captures(inv_line) {
                inv_defines.input = cap.get(1).unwrap().as_str().trim().to_string();
            } else if let Some(cap) = output_regex.captures(inv_line) {
                inv_defines.output = cap.get(1).unwrap().as_str().trim().to_string();
            } else if empty_line_regex.is_match(inv_line) {
            } else {
                eprintln!("Error: Undefined Inv Option");
                eprintln!("Syntax error at line {}", i + 1);
                eprintln!("{}", inv_line);
                panic!("Syntax Error at line {}", i + 1);
            }
        }
        return inv_defines;
    }

    pub fn get_name_mut(&mut self) -> &mut String {
        &mut self.name
    }
    /// Return if any of field is empty.
    pub fn is_empty(&self) -> bool {
        self.name.is_empty() || self.input.is_empty() || self.output.is_empty()
    }
    fn to_gate(&self, input_wire: &String, output_wire: &String) -> Gate {
        let mut inv_gate = Gate::default();
        *inv_gate.get_name_mut() = self.name.clone();
        inv_gate.push_port(PortWire::Wire(self.input.clone(), input_wire.clone()));
        inv_gate.push_port(PortWire::Wire(self.output.clone(), output_wire.clone()));
        inv_gate
    }
}

#[cfg(test)]
mod test {
    use crate::time_expansion::config::ExpansionMethod::Broadside;
    use crate::time_expansion::config::{ExpansionConfig, FFDefinition, InvDefinition};
    use crate::verilog::netlist_serializer::NetlistSerializer;
    use crate::verilog::Verilog;

    #[test]
    fn expansion_config() {
        let ec = ExpansionConfig::from_file("expansion_example.conf").unwrap();
        assert_eq!(ec.expand_method, Some(Broadside));
        assert_eq!(ec.input_file, "b01_net.v");
        assert_eq!(ec.output_file, "b01_bs_net.v");
        assert_eq!(ec.top_module, "b01");
        assert_eq!(ec.clock_pins, vec!["clock", "reset"]);
        assert_eq!(ec.equivalent_check, (false, String::from("FLAG_reg/Q")));
        assert!(!ec.use_primary_io);
        assert_eq!(
            ec.ff_definitions,
            vec![
                FFDefinition {
                    name: String::from("FD2S"),
                    data_in: vec![String::from("D")],
                    data_out: vec![String::from("Q"), String::from("QN")],
                    control: vec![
                        String::from("TI"),
                        String::from("TE"),
                        String::from("CP"),
                        String::from("CD")
                    ],
                },
                FFDefinition {
                    name: String::from("FD2"),
                    data_in: vec![String::from("D")],
                    data_out: vec![String::from("Q"), String::from("QN")],
                    control: vec![String::from("CP"), String::from("CD")],
                },
                FFDefinition {
                    name: String::from("FD1S"),
                    data_in: vec![String::from("D")],
                    data_out: vec![String::from("Q"), String::from("QN")],
                    control: vec![String::from("TI"), String::from("TE"), String::from("CP")],
                },
                FFDefinition {
                    name: String::from("FD1"),
                    data_in: vec![String::from("D")],
                    data_out: vec![String::from("Q"), String::from("QN")],
                    control: vec![String::from("CP")],
                },
            ]
        );
        assert_eq!(
            ec.inv_definition,
            InvDefinition {
                name: String::from("IV"),
                input: String::from("A"),
                output: String::from("Z")
            }
        )
    }

    #[test]
    pub fn extract_combinational_part() {
        let ec = ExpansionConfig::from_file("expansion_example.conf").unwrap();
        let verilog = Verilog::from_config(&ec);
        let m = verilog.get_module(&ec.top_module).unwrap();
        let (c, ppis, ppos) = ec.extract_combinational_part(m);
        eprintln!("{}", c.gen());
        eprintln!("ppis = {:?}", ppis);
        eprintln!("ppos = {:?}", ppos);
    }

    #[test]
    pub fn expand_circuit() {
        let ec = ExpansionConfig::from_file("expansion_example.conf").unwrap();
        let v = ec.time_expand();
        eprintln!("{}", v.gen());
    }

    #[test]
    pub fn atpg_model_time_expand() -> Result<(), String> {
        let ec = ExpansionConfig::from_file("expansion_example.conf").unwrap();
        let v = ec.atpg_model_time_expand()?;
        eprintln!("{}", v.gen());
        Ok(())
    }

    #[test]
    pub fn equivalent_check() -> Result<(), String> {
        let ec = ExpansionConfig::from_file("expansion_example.conf").unwrap();
        let v = ec.equivalent_check()?;
        eprintln!("{}", v.gen());
        Ok(())
    }
}
