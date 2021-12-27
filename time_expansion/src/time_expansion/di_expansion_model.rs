use crate::gen_configured_trait;
use crate::time_expansion::config::{ConfiguredTrait, ExpansionConfigError};
use crate::time_expansion::time_expansion_model::{BroadSideExpansionModel, TimeExpansionModel};
use crate::time_expansion::{ExtractedCombinationalPartModel, TopModule};
use crate::verilog::fault::Fault;
use crate::verilog::netlist_serializer::NetlistSerializer;
use crate::verilog::{Gate, Module, PortWire, Verilog, VerilogError, Wire};
use std::collections::btree_set::BTreeSet;
use std::convert::TryFrom;

pub trait DiExpansionModelTrait: TimeExpansionModel {
    fn c3_suffix() -> &'static str {
        "_c3"
    }
    fn c3_gate_name() -> &'static str {
        "C3"
    }
    fn c3_name(&self) -> String {
        self.combinational_part_name_with_suffix(Self::c3_suffix())
    }

    fn c3_module(&self) -> &Module;
}

#[derive(Debug, Clone)]
pub struct DiExpansionModel {
    combinational_part_model: ExtractedCombinationalPartModel,
    expanded_model: Verilog,
}
impl DiExpansionModel {
    pub fn expanded_model(&self) -> &Verilog {
        &self.expanded_model
    }
}
gen_configured_trait!(DiExpansionModel, combinational_part_model);
impl TopModule for DiExpansionModel {
    fn top_module(&self) -> &Module {
        self.expanded_model()
            .module_by_name(self.top_name().as_str())
            .unwrap()
    }
}
impl TimeExpansionModel for DiExpansionModel {
    fn c1_module(&self) -> &Module {
        self.expanded_model()
            .module_by_name(self.c1_name().as_str())
            .unwrap()
    }
    fn c2_module(&self) -> &Module {
        self.expanded_model()
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
impl DiExpansionModelTrait for DiExpansionModel {
    fn c3_module(&self) -> &Module {
        self.expanded_model()
            .module_by_name(self.c3_name().as_str())
            .unwrap()
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

        for ppi in ppis {
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
            *output_0.name_mut() = format!("{}_restricted", c2_output_name);
            *output_1.name_mut() = format!("{}_restricted", c3_output_name);
            expanded_module.remove_output(&output);
            expanded_module.push_assign(format!("{} = {}", output_0.name(), c2_output_name));
            expanded_module.push_assign(format!("{} = {}", output_1.name(), c3_output_name));
            *output.name_mut() = c3_output_name.clone();
            expanded_module.push_wire(output);
            expanded_module.push_output(output_0.clone());
            expanded_module.push_output(output_1.clone());
        }

        expanded_module.push_gate(String::from(DiExpansionModel::c3_gate_name()), gate_c3);

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
    pub fn insert_restricted_gates(
        top_module: &mut Module,
        outputs: &[PortWire],
        restriction_wire: &Wire,
        fault: &Fault,
        inverted: bool,
        cmb_name: &str,
    ) {
        let assigns = top_module.assigns().clone();
        assigns
            .into_iter()
            .filter(|assign| {
                outputs
                    .iter()
                    .any(|port_wire| assign.ends_with(port_wire.wire()))
            })
            .enumerate()
            .for_each(|(i, res_assign)| {
                let mut res_out = res_assign.split('=').map(|s| s.trim().to_string());
                let top_output = res_out.next().unwrap();
                let module_output = res_out.next().unwrap();
                let ppo_r = format!(
                    "{output}_{location}_{str_stf}",
                    output = module_output,
                    location = fault.sanitized_location(),
                    str_stf = fault.slow_to(),
                );
                let mut restriction_gate = Gate::default();
                *restriction_gate.name_mut() = String::from(if fault.sa_value() ^ inverted {
                    "AN2"
                } else {
                    "OR2"
                });
                {
                    use crate::verilog::PortWire::Wire;
                    restriction_gate
                        .push_port(Wire(String::from('A'), restriction_wire.name().to_string()));
                    restriction_gate.push_port(Wire(String::from('B'), ppo_r.clone()));
                    restriction_gate.push_port(Wire(String::from('Z'), top_output));
                }
                top_module.push_gate(
                    format!(
                        "R{index}_{location}_{str_stf}_{cmb}",
                        index = i + 1,
                        location = fault.sanitized_location(),
                        str_stf = fault.slow_to(),
                        cmb = cmb_name,
                    ),
                    restriction_gate,
                );
                top_module.push_assign(format!("{} = {}", ppo_r, module_output));
                top_module.push_wire(Wire::new_single(ppo_r));
                top_module.remove_assign(&res_assign);
            });
    }
    pub fn atpg_model(&self) -> &Verilog {
        &self.atpg_model
    }
    pub fn equivalent_check(&self) -> Result<(Verilog, Verilog), VerilogError> {
        let mut faulty_model = self.atpg_model().clone();

        let ud_fault = self.cfg_equivalent_check().get(0).unwrap();
        let dt_fault = self.cfg_equivalent_check().get(1).unwrap();

        let c2_imp = faulty_model
            .module_by_name_mut(self.c2_name().as_str())
            .unwrap();
        *c2_imp = c2_imp.insert_stuck_at_fault(self.c2_name().as_str(), ud_fault)?;
        *c2_imp = c2_imp.insert_stuck_at_fault(self.c2_name().as_str(), dt_fault)?;
        let c3_imp = faulty_model
            .module_by_name_mut(self.c3_name().as_str())
            .unwrap();
        *c3_imp = c3_imp.insert_stuck_at_fault(self.c3_name().as_str(), dt_fault)?;

        Ok((self.atpg_model().clone(), faulty_model))
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
impl DiExpansionModelTrait for DiExpansionATPGModel {
    fn c3_module(&self) -> &Module {
        self.atpg_model()
            .module_by_name(self.c3_name().as_str())
            .unwrap()
    }
}
gen_configured_trait!(DiExpansionATPGModel, de_model);
impl TryFrom<DiExpansionModel> for DiExpansionATPGModel {
    type Error = crate::time_expansion::ExpansionConfigError;
    /// insert restricted value gates for generating atpg model
    fn try_from(de_model: DiExpansionModel) -> Result<Self, Self::Error> {
        let mut atpg_model = Verilog::default();
        let mut top_module = de_model.top_module().clone();
        let mut c1_module = de_model.c1_module().clone();
        let c2_module = de_model.c2_module().clone();
        let c3_module = de_model.c3_module().clone();
        if de_model.cfg_equivalent_check().len() < 2 {
            return Err(ExpansionConfigError::ConfigIsUnsatisfied(format!(
                "Faults' count must be 2, but it's {}",
                de_model.cfg_equivalent_check().len()
            )));
        }
        let ud_fault: &crate::verilog::fault::Fault =
            de_model.cfg_equivalent_check().get(0).unwrap();
        let dt_fault: &crate::verilog::fault::Fault =
            de_model.cfg_equivalent_check().get(1).unwrap();

        let ud_observable_port =
            c1_module.add_observation_point(ud_fault.location(), ud_fault.sa_value())?;
        let dt_observable_port =
            c1_module.add_observation_point(dt_fault.location(), dt_fault.sa_value())?;

        // take restriction wire from c1's observable port
        let ud_restriction_wire = Wire::new_single(ud_observable_port.clone());
        let dt_restriction_wire = Wire::new_single(dt_observable_port.clone());
        top_module.push_wire(ud_restriction_wire.clone());
        top_module.push_wire(dt_restriction_wire.clone());
        let c1_gate = top_module
            .gate_mut_by_name(DiExpansionModel::c1_gate_name())
            .unwrap();
        c1_gate.push_port(PortWire::Wire(
            ud_observable_port,
            ud_restriction_wire.name().to_string(),
        ));
        c1_gate.push_port(PortWire::Wire(
            dt_observable_port,
            dt_restriction_wire.name().to_string(),
        ));

        let c2_gate = de_model
            .top_module()
            .gate_by_name(DiExpansionModel::c2_gate_name())
            .unwrap();
        let c3_gate = de_model
            .top_module()
            .gate_by_name(DiExpansionModel::c3_gate_name())
            .unwrap();

        // ud_fault, which will be injected c2
        DiExpansionATPGModel::insert_restricted_gates(
            &mut top_module,
            c2_gate.ports(),
            &ud_restriction_wire,
            ud_fault,
            false,
            "c2",
        );
        DiExpansionATPGModel::insert_restricted_gates(
            &mut top_module,
            c3_gate.ports(),
            &ud_restriction_wire,
            ud_fault,
            true,
            "c3",
        );
        DiExpansionATPGModel::insert_restricted_gates(
            &mut top_module,
            c2_gate.ports(),
            &dt_restriction_wire,
            dt_fault,
            false,
            "c2",
        );
        DiExpansionATPGModel::insert_restricted_gates(
            &mut top_module,
            c3_gate.ports(),
            &dt_restriction_wire,
            dt_fault,
            false,
            "c3",
        );

        atpg_model.push_module(top_module);
        atpg_model.push_module(c1_module);
        atpg_model.push_module(c2_module);
        atpg_model.push_module(c3_module);

        Ok(Self {
            de_model,
            atpg_model,
        })
    }
}
impl NetlistSerializer for DiExpansionATPGModel {
    fn gen(&self) -> String {
        self.atpg_model().gen()
    }
}

#[cfg(test)]
mod test {
    use crate::time_expansion::config::{ExpansionConfig, ExpansionConfigError};
    use crate::time_expansion::di_expansion_model::{DiExpansionATPGModel, DiExpansionModel};
    use crate::time_expansion::time_expansion_model::BroadSideExpansionModel;
    use crate::time_expansion::{ConfiguredModel, ExtractedCombinationalPartModel};
    use std::convert::TryFrom;

    fn test_configured_model() -> Result<ConfiguredModel, ExpansionConfigError> {
        ConfiguredModel::try_from(ExpansionConfig::from_file("expansion.conf")?)
    }
    fn test_di_expansion_model() -> Result<DiExpansionModel, ExpansionConfigError> {
        Ok(DiExpansionModel::from(BroadSideExpansionModel::from(
            ExtractedCombinationalPartModel::from(test_configured_model()?),
        )))
    }
    #[test]
    fn di_expansion_model() -> Result<(), ExpansionConfigError> {
        let _bsd = test_di_expansion_model()?;
        Ok(())
    }
    #[test]
    fn di_expansion_atpg_model() -> Result<(), ExpansionConfigError> {
        let _dam = DiExpansionATPGModel::try_from(test_di_expansion_model()?)?;
        Ok(())
    }
    #[test]
    fn di_expansion_equivalent_check() -> Result<(), ExpansionConfigError> {
        let dam = DiExpansionATPGModel::try_from(test_di_expansion_model()?)?;
        let (_ref_v, _imp_v) = dam.equivalent_check().unwrap();
        Ok(())
    }
}
