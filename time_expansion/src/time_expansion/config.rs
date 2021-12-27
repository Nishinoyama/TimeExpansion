use std::convert::TryFrom;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::iter::Enumerate;
use std::option::Option::Some;

use regex::Regex;

use crate::time_expansion::config::ExpansionMethod::{Broadside, SkewedLoad};
use crate::verilog::fault::Fault;
use crate::verilog::{Gate, ModuleError, PortWire, Verilog, VerilogError};

#[macro_export]
macro_rules! gen_configured_trait {
    ($struct_name:ident, $cfg_field:ident) => {
        impl ConfiguredTrait for $struct_name {
            fn cfg_expand_method(&self) -> &Option<crate::time_expansion::config::ExpansionMethod> {
                self.$cfg_field.cfg_expand_method()
            }
            fn cfg_input_file(&self) -> &str {
                self.$cfg_field.cfg_input_file()
            }
            fn cfg_output_file(&self) -> &str {
                self.$cfg_field.cfg_output_file()
            }
            fn cfg_top_module(&self) -> &str {
                self.$cfg_field.cfg_top_module()
            }
            fn cfg_clock_pins(&self) -> &Vec<String> {
                self.$cfg_field.cfg_clock_pins()
            }
            fn cfg_use_primary_io(&self) -> bool {
                self.$cfg_field.cfg_use_primary_io()
            }
            fn cfg_equivalent_check(&self) -> &Vec<crate::verilog::fault::Fault> {
                self.$cfg_field.cfg_equivalent_check()
            }
            fn cfg_ff_definitions(&self) -> &Vec<crate::time_expansion::config::FFDefinition> {
                self.$cfg_field.cfg_ff_definitions()
            }
            fn cfg_inv_definition(&self) -> &crate::time_expansion::config::InvDefinition {
                self.$cfg_field.cfg_inv_definition()
            }
        }
    };
}

pub trait ConfiguredTrait {
    fn cfg_expand_method(&self) -> &Option<ExpansionMethod>;
    fn cfg_input_file(&self) -> &str;
    fn cfg_output_file(&self) -> &str;
    fn cfg_top_module(&self) -> &str;
    fn cfg_clock_pins(&self) -> &Vec<String>;
    fn cfg_use_primary_io(&self) -> bool;
    fn cfg_equivalent_check(&self) -> &Vec<Fault>;
    fn cfg_ff_definitions(&self) -> &Vec<FFDefinition>;
    fn cfg_inv_definition(&self) -> &InvDefinition;
}

#[derive(Clone, Default, Debug)]
pub struct ExpansionConfig {
    expand_method: Option<ExpansionMethod>,
    input_file: String,
    output_file: String,
    top_module: String,

    clock_pins: Vec<String>,

    use_primary_io: bool,
    equivalent_check: Vec<Fault>,
    ff_definitions: Vec<FFDefinition>,
    inv_definition: InvDefinition,
}

impl ConfiguredTrait for ExpansionConfig {
    fn cfg_expand_method(&self) -> &Option<ExpansionMethod> {
        self.expand_method()
    }
    fn cfg_input_file(&self) -> &str {
        self.input_file()
    }
    fn cfg_output_file(&self) -> &str {
        self.output_file()
    }
    fn cfg_top_module(&self) -> &str {
        self.top_module()
    }
    fn cfg_clock_pins(&self) -> &Vec<String> {
        self.clock_pins()
    }
    fn cfg_use_primary_io(&self) -> bool {
        self.use_primary_io()
    }
    fn cfg_equivalent_check(&self) -> &Vec<Fault> {
        self.equivalent_check()
    }
    fn cfg_ff_definitions(&self) -> &Vec<FFDefinition> {
        self.ff_definitions()
    }
    fn cfg_inv_definition(&self) -> &InvDefinition {
        self.inv_definition()
    }
}

impl ExpansionConfig {
    fn read_file(&self, file_name: &str) -> std::io::Result<Vec<String>> {
        let config_file = File::open(file_name)?;
        let config_buf_reader = BufReader::new(config_file);
        let lines = config_buf_reader
            .lines()
            .map(|line| line.unwrap().split('#').next().unwrap().to_string())
            .collect();
        Ok(lines)
    }
    fn parse_lines(&mut self, lines: Vec<String>) -> Result<(), ExpansionConfigError> {
        let expansion_method_regex = Regex::new(r"\s*expansion-method\s+(\S+)\s*").unwrap();
        let input_verilog_regex = Regex::new(r"\s*input-verilog\s+(\S+)\s*").unwrap();
        let output_verilog_regex = Regex::new(r"\s*output-verilog\s+(\S+)\s*").unwrap();
        let top_module_regex = Regex::new(r"\s*top-module\s+(\S+)\s*").unwrap();
        let clock_pins_regex = Regex::new(r"\s*clock-pins\s+(.+)\s*").unwrap();
        let use_primary_io_regex = Regex::new(r"\s*use-primary-io\s+(.+)\s*").unwrap();
        let multi_ec_regex = Regex::new(r"\s*equivalent-check\s*\{.*").unwrap();
        let equivalent_check_regex = Regex::new(r"\s*equivalent-check\s+(.+)\s*").unwrap();
        let ff_definitions_regex = Regex::new(r"\s*ff\s+([^{]+)\s*\{.*").unwrap();
        let inv_definitions_regex = Regex::new(r"\s*inv\s+([^{]+)\s*\{.*").unwrap();
        let empty_line_regex = Regex::new(r"^\s*$").unwrap();

        let mut line_iter = lines.into_iter().enumerate();
        while let Some((i, line)) = line_iter.next() {
            if let Some(cap) = expansion_method_regex.captures(&line) {
                self.expand_method = ExpansionMethod::from_string(cap.get(1).unwrap().as_str());
            } else if let Some(cap) = input_verilog_regex.captures(&line) {
                self.input_file = cap.get(1).unwrap().as_str().to_string();
            } else if let Some(cap) = output_verilog_regex.captures(&line) {
                self.output_file = cap.get(1).unwrap().as_str().to_string();
            } else if let Some(cap) = top_module_regex.captures(&line) {
                self.top_module = cap.get(1).unwrap().as_str().to_string();
            } else if let Some(cap) = clock_pins_regex.captures(&line) {
                cap.get(1)
                    .unwrap()
                    .as_str()
                    .split(',')
                    .for_each(|pin| self.clock_pins.push(pin.trim().to_string()));
            } else if let Some(cap) = use_primary_io_regex.captures(&line) {
                self.use_primary_io = !cap.get(1).unwrap().as_str().to_lowercase().eq("no");
            } else if let Some(_cap) = multi_ec_regex.captures(&line) {
                let fault_regex = Regex::new(r"\s*(st[rf])\s+(\S+)\s+(\S+).*").unwrap();
                while let Some(fault_cap) =
                    fault_regex.captures(line_iter.next().unwrap().1.as_str())
                {
                    self.equivalent_check.push(Fault::new(
                        fault_cap.get(3).unwrap().as_str().to_string(),
                        fault_cap.get(1).unwrap().as_str().eq("stf"),
                    ))
                }
            } else if let Some(cap) = equivalent_check_regex.captures(&line) {
                let fault_regex = Regex::new(r"\s*(st[rf])\s+(\S+)\s+(\S+).*").unwrap();
                if let Some(fault_cap) = fault_regex.captures(cap.get(1).unwrap().as_str()) {
                    self.equivalent_check.push(Fault::new(
                        fault_cap.get(3).unwrap().as_str().to_string(),
                        fault_cap.get(1).unwrap().as_str().eq("stf"),
                    ))
                } else {
                    return Err(ExpansionConfigError::ConfigSyntaxError(format!(
                        "Error: Equivalent check fault syntax Error at line {}",
                        i + 1
                    )));
                }
            } else if let Some(cap) = ff_definitions_regex.captures(&line) {
                let mut ff_define = FFDefinition::from_file_iter(&mut line_iter)?;
                *ff_define.name_mut() = cap.get(1).unwrap().as_str().trim().to_string();
                self.ff_definitions.push(ff_define);
            } else if let Some(cap) = inv_definitions_regex.captures(&line) {
                let mut inv_define = InvDefinition::from_file_iter(&mut line_iter)?;
                *inv_define.name_mut() = cap.get(1).unwrap().as_str().trim().to_string();
                self.inv_definition = inv_define;
            } else if empty_line_regex.is_match(&line) {
            } else {
                // eprintln!("Error: Undefined Option");
                // eprintln!("Syntax error at line {}", i + 1);
                // eprintln!("{}", line);
                return Err(ExpansionConfigError::ConfigSyntaxError(format!(
                    "Error: Undefined Option. Syntax Error at line {}",
                    i + 1
                )));
            }
        }
        Ok(())
    }
    fn verification(&self) -> Result<(), ExpansionConfigVerificationError> {
        use ExpansionConfigVerificationError::*;
        if self.clock_pins.is_empty() {
            eprintln!("Warning: clock-pins option is blank. (Asynchronous circuit?)");
        }
        if self.expand_method().is_none()
            && self
                .equivalent_check()
                .iter()
                .any(|f| f.location().is_empty())
        {
            Err(UnspecifiedExpansionMethod)
        } else if self.input_file().is_empty() {
            Err(NoInputFile)
        } else if self.output_file().is_empty() {
            Err(NoOutputFile)
        } else if self.top_module().is_empty() {
            Err(UnspecifiedTopModule)
        } else if self.ff_definitions().is_empty() {
            Err(UnspecifiedFFGate)
        } else if self.inv_definition().is_empty() {
            Err(UnspecifiedInvGate)
        } else {
            Ok(())
        }
    }
    /// Generate config from file specifying config.
    /// [`Err`]\([`String`]) if file cannot compiled or is wrong.
    pub fn from_file(file_name: &str) -> Result<Self, ExpansionConfigError> {
        let mut config = Self::default();
        let lines = config.read_file(file_name).unwrap();
        config.parse_lines(lines)?;
        config.verification()?;
        Ok(config)
    }
    /// Returns input file name specified by config.
    pub fn expand_method(&self) -> &Option<ExpansionMethod> {
        &self.expand_method
    }
    pub fn input_file(&self) -> &str {
        &self.input_file
    }
    pub fn output_file(&self) -> &str {
        &self.output_file
    }
    pub fn top_module(&self) -> &str {
        &self.top_module
    }
    pub fn clock_pins(&self) -> &Vec<String> {
        &self.clock_pins
    }
    pub fn use_primary_io(&self) -> bool {
        self.use_primary_io
    }
    pub fn equivalent_check(&self) -> &Vec<Fault> {
        &self.equivalent_check
    }
    pub fn ff_definitions(&self) -> &Vec<FFDefinition> {
        &self.ff_definitions
    }
    pub fn inv_definition(&self) -> &InvDefinition {
        &self.inv_definition
    }
}

impl TryFrom<ExpansionConfig> for Verilog {
    type Error = VerilogError;
    fn try_from(cfg: ExpansionConfig) -> Result<Self, Self::Error> {
        Self::from_file(cfg.input_file())
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
    pub fn from_file_iter(
        line_iter: &mut Enumerate<std::vec::IntoIter<String>>,
    ) -> Result<Self, FFDefinitionError> {
        let mut ff_defines = Self::default();
        let data_in_regex = Regex::new(r"\s*data-in\s+(.+)\s*").unwrap();
        let data_out_regex = Regex::new(r"\s*data-out\s+(.+)\s*").unwrap();
        let control_regex = Regex::new(r"\s*control\s+(.+)\s*").unwrap();
        let empty_line_regex = Regex::new(r"^\s*$").unwrap();

        for (i, ff_line) in line_iter {
            if ff_line.contains('}') {
                break;
            }
            if let Some(cap) = data_in_regex.captures(&ff_line) {
                cap.get(1)
                    .unwrap()
                    .as_str()
                    .split(',')
                    .for_each(|data| ff_defines.data_in.push(data.trim().to_string()));
            } else if let Some(cap) = data_out_regex.captures(&ff_line) {
                cap.get(1)
                    .unwrap()
                    .as_str()
                    .split(',')
                    .for_each(|data| ff_defines.data_out.push(data.trim().to_string()));
            } else if let Some(cap) = control_regex.captures(&ff_line) {
                cap.get(1)
                    .unwrap()
                    .as_str()
                    .split(',')
                    .for_each(|data| ff_defines.control.push(data.trim().to_string()));
            } else if empty_line_regex.is_match(&ff_line) {
            } else {
                return Err(FFDefinitionError::UndefinedOption(format!(
                    "Syntax Error at line {}",
                    i + 1
                )));
            }
        }
        Ok(ff_defines)
    }
    pub fn data_in(&self) -> &Vec<String> {
        &self.data_in
    }
    pub fn data_out(&self) -> &Vec<String> {
        &self.data_out
    }
    pub fn control(&self) -> &Vec<String> {
        &self.control
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }
}

#[derive(Debug)]
pub enum FFDefinitionError {
    UndefinedOption(String),
}

#[derive(Clone, Default, Debug, Eq, PartialEq)]
pub struct InvDefinition {
    name: String,
    input: String,
    output: String,
}

impl InvDefinition {
    pub fn from_file_iter(
        line_iter: &mut Enumerate<std::vec::IntoIter<String>>,
    ) -> Result<Self, InvDefinitionError> {
        let mut inv_defines = Self::default();
        let input_regex = Regex::new(r"\s*input\s+(\w+)\s*").unwrap();
        let output_regex = Regex::new(r"\s*output\s+(\w+)\s*").unwrap();
        let empty_line_regex = Regex::new(r"^\s*$").unwrap();

        for (i, inv_line) in line_iter {
            if inv_line.contains('}') {
                break;
            }
            if let Some(cap) = input_regex.captures(&inv_line) {
                inv_defines.input = cap.get(1).unwrap().as_str().trim().to_string();
            } else if let Some(cap) = output_regex.captures(&inv_line) {
                inv_defines.output = cap.get(1).unwrap().as_str().trim().to_string();
            } else if empty_line_regex.is_match(&inv_line) {
            } else {
                return Err(InvDefinitionError::UndefinedOption(format!(
                    "Syntax Error at line {}",
                    i + 1
                )));
            }
        }
        Ok(inv_defines)
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }
    pub fn input(&self) -> &str {
        &self.input
    }
    pub fn output(&self) -> &str {
        &self.output
    }
    /// Returns if any of field is empty.
    pub fn is_empty(&self) -> bool {
        self.name.is_empty() || self.input.is_empty() || self.output.is_empty()
    }
    /// Generates inv gate with input/output wire.
    pub fn to_gate(&self, input_wire: String, output_wire: String) -> Gate {
        use PortWire::Wire;
        let mut inv_gate = Gate::default();
        *inv_gate.name_mut() = self.name.clone();
        inv_gate.push_port(Wire(self.input.clone(), input_wire));
        inv_gate.push_port(Wire(self.output.clone(), output_wire));
        inv_gate
    }
}

#[derive(Debug)]
pub enum InvDefinitionError {
    UndefinedOption(String),
}

#[derive(Debug)]
pub enum ExpansionConfigError {
    ConfigSyntaxError(String),
    ConfigVerificationError(ExpansionConfigVerificationError),
    ConfigIsUnsatisfied(String),
    FFDefinitionError(FFDefinitionError),
    InvDefinitionError(InvDefinitionError),
    ModuleError(ModuleError),
    VerilogError(VerilogError),
    IOError(std::io::Error),
}

impl From<std::io::Error> for ExpansionConfigError {
    fn from(e: std::io::Error) -> Self {
        Self::IOError(e)
    }
}

impl From<ModuleError> for ExpansionConfigError {
    fn from(e: ModuleError) -> Self {
        Self::ModuleError(e)
    }
}

impl From<VerilogError> for ExpansionConfigError {
    fn from(e: VerilogError) -> Self {
        Self::VerilogError(e)
    }
}

impl From<FFDefinitionError> for ExpansionConfigError {
    fn from(e: FFDefinitionError) -> Self {
        Self::FFDefinitionError(e)
    }
}

impl From<InvDefinitionError> for ExpansionConfigError {
    fn from(e: InvDefinitionError) -> Self {
        Self::InvDefinitionError(e)
    }
}

#[derive(Debug)]
/// Verification error in the configuration file.
pub enum ExpansionConfigVerificationError {
    /// expand-method option must be specified
    UnspecifiedExpansionMethod,
    /// input-file option must be specified
    NoInputFile,
    /// output-file option must be specified
    NoOutputFile,
    /// ff option must be specified at least 1
    UnspecifiedFFGate,
    /// inv option must be specified in the configuration file
    /// or cannot analyze the following NOT gate type specified in inv option.
    UnspecifiedInvGate,
    /// top-module name must be specified
    UnspecifiedTopModule,
}

impl From<ExpansionConfigVerificationError> for ExpansionConfigError {
    fn from(e: ExpansionConfigVerificationError) -> Self {
        Self::ConfigVerificationError(e)
    }
}

#[cfg(test)]
mod test {
    use crate::time_expansion::config::ExpansionMethod::Broadside;
    use crate::time_expansion::config::{
        ExpansionConfig, ExpansionConfigError, FFDefinition, InvDefinition,
    };
    use crate::verilog::fault::Fault;

    #[test]
    fn expansion_config() -> Result<(), ExpansionConfigError> {
        let ec = ExpansionConfig::from_file("expansion.conf")?;
        assert_eq!(ec.expand_method, Some(Broadside));
        assert_eq!(ec.input_file, "b01_net.v");
        assert_eq!(ec.output_file, "b01_bs_net.v");
        assert_eq!(ec.top_module, "b01");
        assert_eq!(ec.clock_pins, vec!["clock", "reset"]);
        assert_eq!(
            ec.equivalent_check,
            vec![
                Fault::new(String::from("U10/B"), false),
                Fault::new(String::from("U28/A"), true),
            ]
        );
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
                        String::from("CD"),
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
                output: String::from("Z"),
            }
        );
        Ok(())
    }
}
