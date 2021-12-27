use std::convert::TryFrom;
use std::env::args;
use std::fs::File;
use std::io::{BufWriter, Write};

use kyiw_time_expansion::time_expansion::config::*;
use kyiw_time_expansion::time_expansion::di_expansion_model::{
    DiExpansionATPGModel, DiExpansionModel,
};
use kyiw_time_expansion::time_expansion::time_expansion_model::BroadSideExpansionModel;
use kyiw_time_expansion::time_expansion::{ConfiguredModel, ExtractedCombinationalPartModel};
use kyiw_time_expansion::verilog::netlist_serializer::NetlistSerializer;

#[allow(unused_must_use)]
fn main() {
    prelude();
}

fn prelude() -> Result<(), ExpansionConfigError> {
    let argv = args().collect::<Vec<String>>();
    let file = argv
        .get(1)
        .cloned()
        .unwrap_or_else(|| String::from("expansion.conf"));
    eprintln!("expanding...");
    let cfg = ExpansionConfig::from_file(file.as_str())?;
    eprintln!("time expanding...");
    let dem = DiExpansionModel::from(BroadSideExpansionModel::from(
        ExtractedCombinationalPartModel::from(ConfiguredModel::try_from(cfg)?),
    ));
    eprintln!("writing to {}...", dem.cfg_output_file());
    let write_file = File::create(dem.cfg_output_file())?;
    let mut buf_writer = BufWriter::new(write_file);
    buf_writer.write_all(dem.expanded_model().gen().as_bytes())?;

    eprintln!("atpg expanding...");
    let dam = DiExpansionATPGModel::try_from(dem)?;
    eprintln!("fault injecting...");
    let (ref_v, imp_v) = dam.equivalent_check()?;
    eprintln!("writing to ref.v...");
    let write_file = File::create("ref.v")?;
    let mut buf_writer = BufWriter::new(write_file);
    buf_writer.write_all(ref_v.gen().as_bytes())?;
    eprintln!("writing to imp.v...");
    let write_file = File::create("imp.v")?;
    let mut buf_writer = BufWriter::new(write_file);
    buf_writer.write_all(imp_v.gen().as_bytes())?;
    eprintln!("done!");
    Ok(())
}
