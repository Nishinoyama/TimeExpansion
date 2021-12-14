use kyiw_time_expansion::time_expansion::config::{ConfiguredTrait, ExpansionConfig};
use kyiw_time_expansion::time_expansion::di_expansion_model::{
    DiExpansionATPGModel, DiExpansionModel,
};
use kyiw_time_expansion::time_expansion::time_expansion_model::BroadSideExpansionModel;
use kyiw_time_expansion::time_expansion::{ConfiguredModel, ExtractedCombinationalPartModel};
use kyiw_time_expansion::verilog::netlist_serializer::NetlistSerializer;
use std::env::args;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::process::exit;

fn main() -> Result<(), String> {
    let argv = args().collect::<Vec<String>>();
    let file = argv
        .get(1)
        .cloned()
        .unwrap_or(String::from("expansion.conf"));
    eprintln!("expanding...");
    let cfg = ExpansionConfig::from_file(file.as_str())?;
    eprintln!("time expanding...");
    let dem = DiExpansionATPGModel::from(DiExpansionModel::from(BroadSideExpansionModel::from(
        ExtractedCombinationalPartModel::from(ConfiguredModel::from(cfg)),
    )));
    eprintln!("fault injecting ...");
    let (ref_v, imp_v) = dem.equivalent_check().unwrap();
    eprintln!("writing to ref.v ...");
    let mut write_file = File::create("ref.v").unwrap();
    let mut buf_writer = BufWriter::new(write_file);
    buf_writer.write(ref_v.gen().as_bytes());
    eprintln!("writing to imp.v ...");
    let mut write_file = File::create("imp.v").unwrap();
    let mut buf_writer = BufWriter::new(write_file);
    buf_writer.write(imp_v.gen().as_bytes());
    eprintln!("done!");
    Ok(())
}
