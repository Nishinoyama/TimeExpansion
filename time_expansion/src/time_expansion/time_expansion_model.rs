use crate::gen_configured_trait;
use crate::time_expansion::config::ConfiguredTrait;
use crate::time_expansion::config::ExpansionMethod::Broadside;
use crate::time_expansion::{ExtractedCombinationalPartModel, TopModule};
use crate::verilog::fault::Fault;
use crate::verilog::{Gate, Module, PortWire, Verilog, Wire};
use std::collections::HashSet;

pub trait TimeExpansionModel: ConfiguredTrait {
    fn c1_suffix() -> &'static str {
        "_c1"
    }
    fn c2_suffix() -> &'static str {
        "_c2"
    }
    fn top_suffix() -> &'static str {
        "_bs"
    }
    fn combinational_part_name_with_suffix(&self, suffix: &str) -> String {
        format!("{}{}", self.cfg_top_module(), suffix)
    }
    fn c1_name(&self) -> String {
        self.combinational_part_name_with_suffix(Self::c1_suffix())
    }
    fn c2_name(&self) -> String {
        self.combinational_part_name_with_suffix(Self::c2_suffix())
    }
    fn top_name(&self) -> String {
        self.combinational_part_name_with_suffix(Self::top_suffix())
    }

    fn c1_module(&self) -> &Module;
    fn c2_module(&self) -> &Module;
    fn top_inputs(&self) -> &HashSet<Wire>;
    fn top_outputs(&self) -> &HashSet<Wire>;
}

pub trait EquivalentCheckModel {
    fn implementation_module(&self) -> &Module;
    fn reference_module(&self) -> &Module;
}

#[derive(Debug, Clone)]
pub struct BroadSideExpansionModel {
    combinational_part_model: ExtractedCombinationalPartModel,
    expanded_model: Verilog,
}
impl BroadSideExpansionModel {
    pub fn combinational_part_model(&self) -> &ExtractedCombinationalPartModel {
        &self.combinational_part_model
    }
    pub fn expanded_model(&self) -> &Verilog {
        &self.expanded_model
    }
}
gen_configured_trait!(BroadSideExpansionModel, combinational_part_model);
impl TopModule for BroadSideExpansionModel {
    fn top_module(&self) -> &Module {
        self.expanded_model
            .module_by_name(self.top_name().as_str())
            .unwrap()
    }
}
impl TimeExpansionModel for BroadSideExpansionModel {
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
impl From<ExtractedCombinationalPartModel> for BroadSideExpansionModel {
    /// Generates board side time expansion model from full scan designed circuitry module.
    /// if not `use_primary_io`, primary inputs will be restricted and primary outputs will be masked.
    fn from(combinational_part_model: ExtractedCombinationalPartModel) -> Self {
        let c1_module = combinational_part_model
            .extracted_module
            .clone_with_name_prefix(BroadSideExpansionModel::c1_suffix());
        let c2_module = combinational_part_model
            .extracted_module
            .clone_with_name_prefix(BroadSideExpansionModel::c2_suffix());
        let ppis = combinational_part_model.pseudo_primary_inputs();
        let ppos = combinational_part_model.pseudo_primary_outputs();
        let pis = combinational_part_model.primary_inputs();
        let pos = combinational_part_model.primary_outputs();
        let mut gate_c1 = c1_module.to_gate();
        let mut gate_c2 = c2_module.to_gate();
        let mut expanded_module = Module::new_with_name(format!(
            "{}{}",
            combinational_part_model.cfg_top_module(),
            BroadSideExpansionModel::top_suffix()
        ));

        // connect bs_model inputs to c1 pi, ppis
        for mut input in pis.iter().chain(ppis).cloned() {
            let c1_input_name = format!("{}{}", input.name(), BroadSideExpansionModel::c1_suffix());
            *gate_c1.port_by_name_mut(input.name()).unwrap().wire_mut() = c1_input_name.clone();
            *input.name_mut() = c1_input_name.clone();
            expanded_module.push_input(input);
        }

        // set c1 ppos
        for mut ppo in ppos.iter().cloned() {
            let c1_output_name = format!("{}{}", ppo.name(), BroadSideExpansionModel::c1_suffix());
            *gate_c1.port_by_name_mut(ppo.name()).unwrap().wire_mut() = c1_output_name.clone();
            *ppo.name_mut() = c1_output_name.clone();
            expanded_module.push_wire(ppo);
        }
        // remove c1 pos
        for po in pos {
            gate_c1.take_port_by_name(po.name());
        }

        // chain c1 ppo to c2 ppi
        for (ppi, ppo) in combinational_part_model.pseudo_primary_ios().iter() {
            let c1_ppo_name = format!("{}{}", ppo.name(), BroadSideExpansionModel::c1_suffix());
            let c2_ppi_name = format!("{}{}", ppi.name(), BroadSideExpansionModel::c2_suffix());
            expanded_module.push_assign(format!("{} = {}", c2_ppi_name, c1_ppo_name));
        }

        // chain c1 pi wires (bs_model inputs if use_primary_io) to c2 pi
        for mut pi in pis.iter().cloned() {
            let c1_pi_name = format!("{}{}", pi.name(), BroadSideExpansionModel::c1_suffix());
            let c2_pi_name = format!("{}{}", pi.name(), BroadSideExpansionModel::c2_suffix());
            *gate_c2.port_by_name_mut(pi.name()).unwrap().wire_mut() = c2_pi_name.clone();
            if combinational_part_model.cfg_use_primary_io() {
                *pi.name_mut() = c2_pi_name;
                expanded_module.push_input(pi);
            } else {
                expanded_module.push_assign(format!("{} = {}", c2_pi_name, c1_pi_name));
                *pi.name_mut() = c2_pi_name;
                expanded_module.push_wire(pi);
            }
        }
        for ppi in ppis {
            let c2_input_name = format!("{}{}", ppi.name(), BroadSideExpansionModel::c2_suffix());
            *gate_c2.port_by_name_mut(ppi.name()).unwrap().wire_mut() = c2_input_name.clone();
        }

        // chain c2 ppos (and pos if use_primary_io) to bs_model outputs
        // remove c2 pos if not use_primary_io
        for mut output in c1_module.outputs().iter().cloned() {
            let c2_output_name =
                format!("{}{}", output.name(), BroadSideExpansionModel::c2_suffix());
            *gate_c2.port_by_name_mut(output.name()).unwrap().wire_mut() = c2_output_name.clone();
            if combinational_part_model.cfg_use_primary_io()
                || ppos.iter().any(|ppo| output.name().contains(ppo.name()))
            {
                expanded_module.push_assign(format!("{} = {}", output.name(), c2_output_name));
                expanded_module.push_output(output.clone());
                *output.name_mut() = c2_output_name.clone();
                expanded_module.push_wire(output);
            } else {
                gate_c2.take_port_by_name(output.name());
            }
        }

        expanded_module.push_gate(String::from("c1"), gate_c1);
        expanded_module.push_gate(String::from("c2"), gate_c2);

        let mut verilog = Verilog::default();
        verilog.push_module(expanded_module);
        verilog.push_module(c1_module);
        verilog.push_module(c2_module);
        Self {
            combinational_part_model,
            expanded_model: verilog,
        }
    }
}

#[derive(Debug)]
pub struct BroadSideExpansionATPGModel {
    bs_model: BroadSideExpansionModel,
    atpg_model: Verilog,
}
impl BroadSideExpansionATPGModel {
    fn equivalent_check(&self) -> Result<Verilog, String> {
        let mut atpg_model = self.atpg_model.clone();
        let bs_top_name = self.bs_model.top_name();

        // build bs_ref and bs_imp
        // replace bs_imp's c2 gate with c2_imp
        let bs_ref = atpg_model.module_by_name_mut(&bs_top_name).unwrap();
        *bs_ref.name_mut() = format!("{}_ref", bs_top_name);
        let mut bs_imp = bs_ref.clone();
        *bs_imp.name_mut() = format!("{}_imp", bs_top_name);
        *bs_imp
            .gate_mut_by_name(&String::from("c2"))
            .unwrap()
            .name_mut() = format!("{}_imp", self.c2_module().name());
        atpg_model.push_module(bs_imp);

        // insert stuck-at-fault into c2
        let c2_ref = atpg_model
            .module_by_name_mut(&self.c2_module().name())
            .unwrap();
        let c2_imp = c2_ref
            .insert_stuck_at_fault(
                format!("{}_imp", self.c2_module().name()),
                &self.cfg_equivalent_check(),
            )
            .ok()
            .unwrap();
        atpg_model.push_module(c2_imp);

        Ok(atpg_model)
    }
}
impl TopModule for BroadSideExpansionATPGModel {
    fn top_module(&self) -> &Module {
        self.bs_model.top_module()
    }
}
impl TimeExpansionModel for BroadSideExpansionATPGModel {
    fn c1_module(&self) -> &Module {
        self.bs_model.c1_module()
    }
    fn c2_module(&self) -> &Module {
        self.bs_model.c2_module()
    }
    fn top_inputs(&self) -> &HashSet<Wire> {
        self.bs_model.top_inputs()
    }
    fn top_outputs(&self) -> &HashSet<Wire> {
        self.bs_model.top_outputs()
    }
}
gen_configured_trait!(BroadSideExpansionATPGModel, bs_model);
impl From<BroadSideExpansionModel> for BroadSideExpansionATPGModel {
    /// insert restricted value gates for generating atpg model
    fn from(bs_model: BroadSideExpansionModel) -> Self {
        let mut atpg_model = Verilog::default();

        // gen observable wire in c1 for restriction
        let mut c1_module = bs_model.c1_module().clone();
        let observable_wire = c1_module
            .add_observation_point(&bs_model.cfg_equivalent_check().location().to_string())
            .unwrap();

        // take restriction wire from c1
        let mut top_module = bs_model.top_module().clone();
        let restriction_wire = observable_wire;
        top_module.push_wire(Wire::new_single(restriction_wire.clone()));
        let c1_gate = top_module.gate_mut_by_name(&String::from("c1")).unwrap();
        c1_gate.push_port(PortWire::Wire(
            restriction_wire.clone(),
            restriction_wire.clone(),
        ));

        let c2_outputs = bs_model.c2_module().outputs().clone();
        let restricted_outputs = c2_outputs
            .into_iter()
            .filter_map(|ppo| {
                top_module
                    .assigns()
                    .iter()
                    .find(|assign| assign.ends_with(&format!("{}_c2", ppo.name())))
            })
            .cloned()
            .collect::<Vec<_>>();

        restricted_outputs
            .into_iter()
            .enumerate()
            .for_each(|(i, res_assign)| {
                let mut res_out = res_assign.split("=").map(|s| s.trim().to_string());
                let po = res_out.next().unwrap();
                let ppo_c2 = res_out.next().unwrap();
                let ppo_r = format!(
                    "{}_{}",
                    ppo_c2,
                    bs_model.cfg_equivalent_check().location().replace("/", "_")
                );
                let mut restriction_gate = Gate::default();
                *restriction_gate.name_mut() =
                    String::from(if bs_model.cfg_equivalent_check().sa_value() {
                        "AN2"
                    } else {
                        "OR2"
                    });
                {
                    use crate::verilog::PortWire::Wire;
                    restriction_gate.push_port(Wire(String::from('A'), restriction_wire.clone()));
                    restriction_gate.push_port(Wire(String::from('B'), ppo_r.clone()));
                    restriction_gate.push_port(Wire(String::from('Z'), po.clone()));
                }
                top_module.push_gate(
                    format!(
                        "R{}_{}",
                        i + 1,
                        bs_model.cfg_equivalent_check().location().replace("/", "_")
                    ),
                    restriction_gate,
                );
                top_module.push_assign(format!("{} = {}", ppo_r, ppo_c2));
                top_module.push_wire(Wire::new_single(ppo_r));
                top_module.remove_assign(&res_assign);
            });

        let c2_module = bs_model.c2_module().clone();

        atpg_model.push_module(top_module);
        atpg_model.push_module(c1_module);
        atpg_model.push_module(c2_module);

        Self {
            bs_model,
            atpg_model,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::time_expansion::config::ExpansionConfig;
    use crate::time_expansion::time_expansion_model::{
        BroadSideExpansionATPGModel, BroadSideExpansionModel,
    };
    use crate::time_expansion::{ConfiguredModel, ExtractedCombinationalPartModel};
    use crate::verilog::netlist_serializer::NetlistSerializer;

    fn test_configured_model() -> ConfiguredModel {
        ConfiguredModel::from(ExpansionConfig::from_file("expansion_example.conf").unwrap())
    }

    #[test]
    pub fn broad_side_expand() {
        let bs = BroadSideExpansionModel::from(ExtractedCombinationalPartModel::from(
            test_configured_model(),
        ));
        eprintln!("{}", bs.expanded_model().gen());
    }

    #[test]
    pub fn broad_side_expand_atpg() {
        let bs = BroadSideExpansionModel::from(ExtractedCombinationalPartModel::from(
            test_configured_model(),
        ));
        let atpg = BroadSideExpansionATPGModel::from(bs);
        eprintln!("{}", atpg.atpg_model.gen());
    }

    #[test]
    pub fn equivalent_check_test() {
        let bs = BroadSideExpansionModel::from(ExtractedCombinationalPartModel::from(
            test_configured_model(),
        ));
        let atpg = BroadSideExpansionATPGModel::from(bs);
        let ec = atpg.equivalent_check().unwrap();
        eprintln!("{}", ec.gen());
    }
}
