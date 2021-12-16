use kyiw_time_expansion::time_expansion::config::*;
use kyiw_time_expansion::time_expansion::di_expansion_model::*;
use kyiw_time_expansion::time_expansion::time_expansion_model::BroadSideExpansionModel;
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
        .unwrap_or(String::from("expansion.conf"));
    eprintln!("expanding...");
    let cfg = ExpansionConfig::from_file(file.as_str())?;
    eprintln!("time expanding...");
    let dem = DiExpansionModel::from(BroadSideExpansionModel::from(
        ExtractedCombinationalPartModel::from(ConfiguredModel::from(cfg)),
    ));
    eprintln!("writing to {} ...", dem.cfg_output_file());
    let write_file = File::create(dem.cfg_output_file())?;
    let mut buf_writer = BufWriter::new(write_file);
    buf_writer.write(dem.expanded_model().gen().as_bytes())?;

    let dam = DiExpansionATPGModel::from(dem);
    eprintln!("fault injecting ...");
    let (ref_v, imp_v) = dam.equivalent_check()?;
    eprintln!("writing to ref.v ...");
    let write_file = File::create("ref.v")?;
    let mut buf_writer = BufWriter::new(write_file);
    buf_writer.write(ref_v.gen().as_bytes())?;
    eprintln!("writing to imp.v ...");
    let write_file = File::create("imp.v")?;
    let mut buf_writer = BufWriter::new(write_file);
    buf_writer.write(imp_v.gen().as_bytes())?;
    eprintln!("done!");
    Ok(())
}
