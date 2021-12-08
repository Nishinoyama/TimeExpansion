use crate::gen_configured_trait;
use crate::time_expansion::config::ConfiguredTrait;
use crate::time_expansion::time_expansion_model::{BroadSideExpansionModel, TimeExpansionModel};
use crate::time_expansion::{ExtractedCombinationalPartModel, TopModule};
use crate::verilog::{Module, Verilog, Wire};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct DiExpansionModel {
    combinational_part_model: ExtractedCombinationalPartModel,
    expanded_model: Verilog,
}
impl DiExpansionModel {
    pub fn expanded_model(&self) -> &Verilog {
        &self.expanded_model
    }
    fn combinational_part_name_with_suffix(&self, suffix: &str) -> String {
        format!("{}{}", self.cfg_top_module(), suffix)
    }
    fn c1_name(&self) -> String {
        self.combinational_part_name_with_suffix("_cmb_c1")
    }
    fn c2_name(&self) -> String {
        self.combinational_part_name_with_suffix("_cmb_c2")
    }
    fn c3_name(&self) -> String {
        self.combinational_part_name_with_suffix("_cmb_c3")
    }
    fn top_name(&self) -> String {
        self.combinational_part_name_with_suffix("_bs")
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
    fn top_inputs(&self) -> &HashSet<Wire> {
        self.top_module().inputs()
    }
    fn top_outputs(&self) -> &HashSet<Wire> {
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
            .clone_with_name_prefix("_cmb_c3");
        let ppis = combinational_part_model.pseudo_primary_inputs();
        let ppos = combinational_part_model.pseudo_primary_outputs();
        let pis = combinational_part_model.primary_inputs();
        let pos = combinational_part_model.primary_outputs();
        let mut gate_c3 = c3_module.to_gate();

        // chain c1 ppo to c3 ppi
        for (ppi, ppo) in combinational_part_model.pseudo_primary_ios().iter() {
            expanded_module.push_assign(format!("{}_c3 = {}_c1", ppi.name(), ppo.name()));
        }

        // chain c1 pi wires (bs_model inputs if use_primary_io) to c3 pi
        for mut pi in pis.iter().cloned() {
            let c3_input_name = format!("{}_c3", pi.name());
            *gate_c3.port_by_name_mut(pi.name()).unwrap().wire_mut() = c3_input_name.clone();
            if combinational_part_model.cfg_use_primary_io() {
                *pi.name_mut() = c3_input_name;
                expanded_module.push_input(pi);
            } else {
                expanded_module.push_assign(format!("{}_c3 = {}_c1", pi.name(), pi.name()));
                *pi.name_mut() = c3_input_name;
                expanded_module.push_wire(pi);
            }
        }

        for ppi in ppis.iter().cloned() {
            let c3_input_name = format!("{}_c3", ppi.name());
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
            let c2_output_name = format!("{}_c2", output.name());
            let c3_output_name = format!("{}_c3", output.name());
            expanded_module.remove_assign(&format!("{} = {}", output.name(), c2_output_name));
            *gate_c3.port_by_name_mut(output.name()).unwrap().wire_mut() = c3_output_name.clone();
            *output.name_mut() = c3_output_name.clone();
            *output_0.name_mut() = format!("{}_0", output.name());
            *output_1.name_mut() = format!("{}_1", output.name());
            expanded_module.remove_output(&output);
            expanded_module.push_assign(format!("{} = {}", output_0.name(), c2_output_name));
            expanded_module.push_assign(format!("{} = {}", output_1.name(), c3_output_name));
            expanded_module.push_output(output.clone());
            expanded_module.push_output(output.clone());
            expanded_module.push_wire(output);
            expanded_module.push_wire(output_0);
            expanded_module.push_wire(output_1);
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

#[derive(Debug)]
pub struct BroadSideDiExpansionATPGModel {
    bsd_model: DiExpansionModel,
}

#[cfg(test)]
mod test {
    use crate::time_expansion::config::{ConfiguredTrait, ExpansionConfig};
    use crate::time_expansion::di_expansion_model::DiExpansionModel;
    use crate::time_expansion::time_expansion_model::BroadSideExpansionModel;
    use crate::time_expansion::{ConfiguredModel, ExtractedCombinationalPartModel};
    use crate::verilog::netlist_serializer::NetlistSerializer;
    use std::fs::File;
    use std::io::Write;

    fn test_configured_model() -> ConfiguredModel {
        ConfiguredModel::from(ExpansionConfig::from_file("expansion_example.conf").unwrap())
    }
    #[test]
    fn di_expansion_model() -> std::io::Result<()> {
        let bsd = DiExpansionModel::from(BroadSideExpansionModel::from(
            ExtractedCombinationalPartModel::from(test_configured_model()),
        ));
        let mut file = File::create(bsd.cfg_output_file())?;
        file.write(bsd.expanded_model().gen().as_bytes())?;
        Ok(())
    }
}
