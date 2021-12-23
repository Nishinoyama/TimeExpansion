use kyiw_time_expansion::time_expansion::config::*;
use kyiw_time_expansion::time_expansion::time_expansion_model::{
    BroadSideExpansionATPGModel, BroadSideExpansionModel,
};
use kyiw_time_expansion::time_expansion::{ConfiguredModel, ExtractedCombinationalPartModel};
use kyiw_time_expansion::verilog::netlist_serializer::NetlistSerializer;
use std::env::args;
use std::fs::File;
use std::io::{BufWriter, Write};

fn main() -> Result<(), ExpansionConfigError> {
    let argv = args().collect::<Vec<String>>();
    let file = argv
        .get(1)
        .cloned()
        .unwrap_or_else(|| String::from("expansion.conf"));
    eprintln!("expanding...");
    let cfg = ExpansionConfig::from_file(file.as_str())?;
    eprintln!("time expanding...");
    let bem = BroadSideExpansionModel::from(ExtractedCombinationalPartModel::from(
        ConfiguredModel::try_from(cfg)?,
    ));
    eprintln!("writing to {}...", bem.cfg_output_file());
    let write_file = File::create(bem.cfg_output_file())?;
    let mut buf_writer = BufWriter::new(write_file);
    buf_writer.write_all(bem.expanded_model().gen().as_bytes())?;

    eprintln!("atpg expanding...");
    let bam = BroadSideExpansionATPGModel::try_from(bem)?;
    eprintln!("fault injecting...");
    let (ref_v, imp_v) = bam.equivalent_check()?;
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
