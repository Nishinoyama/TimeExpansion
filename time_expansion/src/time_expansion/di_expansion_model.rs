use crate::gen_configured_trait;
use crate::time_expansion::config::ConfiguredTrait;
use crate::time_expansion::time_expansion_model::{BroadSideExpansionModel, TimeExpansionModel};
use crate::time_expansion::{ExtractedCombinationalPartModel, TopModule};
use crate::verilog::netlist_serializer::NetlistSerializer;
use crate::verilog::{Gate, Module, ModuleError, PortWire, Verilog, Wire};
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct DiExpansionModel {
    combinational_part_model: ExtractedCombinationalPartModel,
    expanded_model: Verilog,
}
impl DiExpansionModel {
    pub fn expanded_model(&self) -> &Verilog {
        &self.expanded_model
    }
    fn sa0_suffix() -> &'static str {
        "_sa0"
    }
    fn sa1_suffix() -> &'static str {
        "_sa1"
    }
    fn c3_suffix() -> &'static str {
        "_c3"
    }
    fn c3_name(&self) -> String {
        self.combinational_part_name_with_suffix(Self::c3_suffix())
    }
    fn c3_module(&self) -> &Module {
        self.expanded_model()
            .module_by_name(self.c3_name().as_str())
            .unwrap()
    }
}
gen_configured_trait!(DiExpansionModel, combinational_part_model);
impl TopModule for DiExpansionModel {
    fn top_module(&self) -> &Module {
        self.expanded_model
            .module_by_name(self.top_name().as_str())
            .unwrap()
    }
}
impl TimeExpansionModel for DiExpansionModel {
    fn c1_module(&self) -> &Module {
        self.expanded_model
            .module_by_name(self.c1_name().as_str())
            .unwrap()
    }
    fn c2_module(&self) -> &Module {
        self.expanded_model
            .module_by_name(self.c2_name().as_str())
            .unwrap()
    }
    fn top_inputs(&self) -> &BTreeSet<Wire> {
        self.top_module().inputs()
    }
    fn top_outputs(&self) -> &BTreeSet<Wire> {
        self.top_module().outputs()
    }
}
impl From<BroadSideExpansionModel> for DiExpansionModel {
    /// Generates board side time expansion model from full scan designed circuitry module.
    /// if not `use_primary_io`, primary inputs will be restricted and primary outputs will be masked.
    fn from(bs_model: BroadSideExpansionModel) -> Self {
        let combinational_part_model = bs_model.combinational_part_model().clone();
        let mut expanded_module = bs_model.top_module().clone();
        let c1_module = bs_model.c1_module().clone();
        let c2_module = bs_model.c2_module().clone();
        let c3_module = combinational_part_model
            .extracted_module()
            .clone_with_name_prefix(DiExpansionModel::c3_suffix());
        let ppis = combinational_part_model.pseudo_primary_inputs();
        let ppos = combinational_part_model.pseudo_primary_outputs();
        let pis = combinational_part_model.primary_inputs();
        let pos = combinational_part_model.primary_outputs();
        let mut gate_c3 = c3_module.to_gate();

        // chain c1 ppo to c3 ppi
        for (ppi, ppo) in combinational_part_model.pseudo_primary_ios().iter() {
            let c3_ppi_name = format!("{}{}", ppi.name(), DiExpansionModel::c3_suffix());
            let c1_ppo_name = format!("{}{}", ppo.name(), DiExpansionModel::c1_suffix());
            expanded_module.push_assign(format!("{} = {}", c3_ppi_name, c1_ppo_name));
        }

        // chain c1 pi wires (bs_model inputs if use_primary_io) to c3 pi
        for mut pi in pis.iter().cloned() {
            let c1_pi_name = format!("{}{}", pi.name(), DiExpansionModel::c1_suffix());
            let c3_pi_name = format!("{}{}", pi.name(), DiExpansionModel::c3_suffix());
            *gate_c3.port_by_name_mut(pi.name()).unwrap().wire_mut() = c3_pi_name.clone();
            if combinational_part_model.cfg_use_primary_io() {
                *pi.name_mut() = c3_pi_name;
                expanded_module.push_input(pi);
            } else {
                expanded_module.push_assign(format!("{} = {}", c3_pi_name, c1_pi_name));
                *pi.name_mut() = c3_pi_name;
                expanded_module.push_wire(pi);
            }
        }

        for ppi in ppis.iter().cloned() {
            let c3_input_name = format!("{}{}", ppi.name(), DiExpansionModel::c3_suffix());
            *gate_c3.port_by_name_mut(ppi.name()).unwrap().wire_mut() = c3_input_name.clone();
        }

        // chain c3 ppos (and pos if use_primary_io) to bs_model outputs
        // remove c3 pos if not use_primary_io
        // change c2 (p)pos to bs_model outputs' name
        let ppos_and_pos = if !combinational_part_model.cfg_use_primary_io() {
            for po in pos.iter() {
                gate_c3.take_port_by_name(po.name());
            }
            ppos.iter().collect::<Vec<_>>()
        } else {
            ppos.iter().chain(pos).collect()
        };
        for mut output in ppos_and_pos.into_iter().cloned() {
            let mut output_0 = output.clone();
            let mut output_1 = output.clone();
            let c2_output_name = format!("{}{}", output.name(), DiExpansionModel::c2_suffix());
            let c3_output_name = format!("{}{}", output.name(), DiExpansionModel::c3_suffix());
            expanded_module.remove_assign(&format!("{} = {}", output.name(), c2_output_name));
            *gate_c3.port_by_name_mut(output.name()).unwrap().wire_mut() = c3_output_name.clone();
            *output_0.name_mut() = format!("{}{}", output.name(), DiExpansionModel::sa0_suffix());
            *output_1.name_mut() = format!("{}{}", output.name(), DiExpansionModel::sa1_suffix());
            expanded_module.remove_output(&output);
            expanded_module.push_assign(format!("{} = {}", output_0.name(), c2_output_name));
            expanded_module.push_assign(format!("{} = {}", output_1.name(), c3_output_name));
            *output.name_mut() = c3_output_name.clone();
            expanded_module.push_wire(output);
            expanded_module.push_output(output_0.clone());
            expanded_module.push_output(output_1.clone());
        }

        expanded_module.push_gate(String::from("c3"), gate_c3);

        let mut verilog = Verilog::default();
        verilog.push_module(expanded_module);
        verilog.push_module(c1_module);
        verilog.push_module(c2_module);
        verilog.push_module(c3_module);

        Self {
            combinational_part_model,
            expanded_model: verilog,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiExpansionATPGModel {
    de_model: DiExpansionModel,
    atpg_model: Verilog,
}
impl DiExpansionATPGModel {
    pub fn atpg_model(&self) -> &Verilog {
        &self.atpg_model
    }
    fn sa0_suffix() -> &'static str {
        "_sa0"
    }
    fn sa1_suffix() -> &'static str {
        "_sa1"
    }
    fn c3_suffix() -> &'static str {
        "_c3"
    }
    fn c3_name(&self) -> String {
        self.combinational_part_name_with_suffix(Self::c3_suffix())
    }
    fn c3_module(&self) -> &Module {
        self.atpg_model()
            .module_by_name(self.c3_name().as_str())
            .unwrap()
    }
    fn equivalent_check(&self) -> Result<(Verilog, Verilog), ModuleError> {
        let mut faulty_model = self.atpg_model.clone();

        // build bs_ref and bs_imp
        // replace bs_imp's c2 gate with c2_imp
        for fault in self.cfg_equivalent_check() {
            let c_name = if fault.sa_value() {
                self.c3_name()
            } else {
                self.c2_name()
            };
            let c_imp = faulty_model.module_by_name_mut(c_name.as_str()).unwrap();
            *c_imp = c_imp.insert_stuck_at_fault(c_name.as_str(), fault)?;
        }

        Ok((self.atpg_model.clone(), faulty_model))
    }
}
impl TopModule for DiExpansionATPGModel {
    fn top_module(&self) -> &Module {
        self.de_model.top_module()
    }
}
impl TimeExpansionModel for DiExpansionATPGModel {
    fn c1_module(&self) -> &Module {
        self.de_model.c1_module()
    }
    fn c2_module(&self) -> &Module {
        self.de_model.c2_module()
    }
    fn top_inputs(&self) -> &BTreeSet<Wire> {
        self.de_model.top_inputs()
    }
    fn top_outputs(&self) -> &BTreeSet<Wire> {
        self.de_model.top_outputs()
    }
}
gen_configured_trait!(DiExpansionATPGModel, de_model);
impl From<DiExpansionModel> for DiExpansionATPGModel {
    /// insert restricted value gates for generating atpg model
    fn from(de_model: DiExpansionModel) -> Self {
        let mut atpg_model = Verilog::default();
        let mut top_module = de_model.top_module().clone();
        let mut c1_module = de_model.c1_module().clone();
        let c2_module = de_model.c2_module().clone();
        let c3_module = de_model.c3_module().clone();

        de_model.cfg_equivalent_check().iter().for_each(|ec_fault| {
            // gen observable wire in c1 for restriction
            let observable_port = c1_module
                .add_observation_point(ec_fault.location(), ec_fault.sa_value())
                .unwrap();

            // take restriction wire from c1's observable port
            let restriction_wire = observable_port.clone();
            top_module.push_wire(Wire::new_single(observable_port.clone()));
            let c1_gate = top_module.gate_mut_by_name(&String::from("c1")).unwrap();
            c1_gate.push_port(PortWire::Wire(
                observable_port.clone(),
                restriction_wire.clone(),
            ));

            let restricted_cmb_gate = de_model
                .top_module()
                .gate_by_name(if ec_fault.sa_value() { "c3" } else { "c2" })
                .unwrap();
            let restricted_assigns = restricted_cmb_gate
                .ports()
                .into_iter()
                .filter_map(|port_wire| {
                    top_module
                        .assigns()
                        .iter()
                        .find(|assign| assign.ends_with(port_wire.wire()))
                })
                .cloned()
                .collect::<Vec<_>>();

            restricted_assigns
                .into_iter()
                .enumerate()
                .for_each(|(i, res_assign)| {
                    let mut res_out = res_assign.split("=").map(|s| s.trim().to_string());
                    let top_output = res_out.next().unwrap();
                    let module_output = res_out.next().unwrap();
                    let ppo_r = format!(
                        "{}_{}_{}",
                        module_output,
                        ec_fault.location().replace("/", "_"),
                        ec_fault.slow_to()
                    );
                    let mut restriction_gate = Gate::default();
                    *restriction_gate.name_mut() =
                        String::from(if ec_fault.sa_value() { "AN2" } else { "OR2" });
                    {
                        use crate::verilog::PortWire::Wire;
                        restriction_gate
                            .push_port(Wire(String::from('A'), restriction_wire.clone()));
                        restriction_gate.push_port(Wire(String::from('B'), ppo_r.clone()));
                        restriction_gate.push_port(Wire(String::from('Z'), top_output.clone()));
                    }
                    top_module.push_gate(
                        format!(
                            "R{}_{}_{}",
                            i + 1,
                            ec_fault.location().replace("/", "_"),
                            ec_fault.slow_to()
                        ),
                        restriction_gate,
                    );
                    top_module.push_assign(format!("{} = {}", ppo_r, module_output));
                    top_module.push_wire(Wire::new_single(ppo_r));
                    top_module.remove_assign(&res_assign);
                });
        });

        atpg_model.push_module(top_module);
        atpg_model.push_module(c1_module);
        atpg_model.push_module(c2_module);
        atpg_model.push_module(c3_module);

        Self {
            de_model,
            atpg_model,
        }
    }
}
impl NetlistSerializer for DiExpansionATPGModel {
    fn gen(&self) -> String {
        self.atpg_model.gen()
    }
}

#[cfg(test)]
mod test {
    use crate::time_expansion::config::{ConfiguredTrait, ExpansionConfig};
    use crate::time_expansion::di_expansion_model::{DiExpansionATPGModel, DiExpansionModel};
    use crate::time_expansion::time_expansion_model::BroadSideExpansionModel;
    use crate::time_expansion::{ConfiguredModel, ExtractedCombinationalPartModel};
    use crate::verilog::netlist_serializer::NetlistSerializer;
    use std::fs::File;
    use std::io::{BufWriter, Write};

    fn test_configured_model() -> ConfiguredModel {
        ConfiguredModel::from(ExpansionConfig::from_file("expansion_example.conf").unwrap())
    }
    fn test_di_expansion_model() -> DiExpansionModel {
        DiExpansionModel::from(BroadSideExpansionModel::from(
            ExtractedCombinationalPartModel::from(test_configured_model()),
        ))
    }
    fn write_file<T: ConfiguredTrait>(model: &T, bytes: &[u8]) -> std::io::Result<()> {
        let mut file = File::create(model.cfg_output_file())?;
        writer(file, bytes)
    }
    fn writer(mut file: File, bytes: &[u8]) -> std::io::Result<()> {
        let mut writer = BufWriter::new(file);
        writer.write(bytes)?;
        Ok(())
    }
    #[test]
    fn di_expansion_model() -> std::io::Result<()> {
        let bsd = test_di_expansion_model();
        write_file(&bsd, bsd.expanded_model().gen().as_bytes())
    }
    #[test]
    fn di_expansion_atpg_model() -> std::io::Result<()> {
        let dam = DiExpansionATPGModel::from(test_di_expansion_model());
        write_file(&dam, dam.atpg_model().gen().as_bytes())
    }
    #[test]
    fn di_expansion_equivalent_check() -> std::io::Result<()> {
        let dam = DiExpansionATPGModel::from(test_di_expansion_model());
        let (ref_v, imp_v) = dam.equivalent_check().unwrap();
        writer(File::create("b01_de_ref.v")?, ref_v.gen().as_bytes())?;
        writer(File::create("b01_de_imp.v")?, imp_v.gen().as_bytes())
    }
}
