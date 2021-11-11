pub trait NetlistSerializer {
    fn gen(&self) -> String;
    fn multi_gen<T: IntoIterator<Item = String> + Clone>(iterable: &T, joiner: &str) -> String {
        iterable
            .clone()
            .into_iter()
            .map(|s| s)
            .collect::<Vec<String>>()
            .join(joiner)
    }
}

#[cfg(test)]
mod test {
    use crate::time_expansion::config::ExpansionConfig;
    use crate::verilog::netlist_serializer::NetlistSerializer;
    use crate::verilog::verilog::Verilog;

    #[test]
    fn expansion_config() {
        let ec = ExpansionConfig::from_file("expansion_example.conf").unwrap();
        let verilog = Verilog::from_config(&ec);
        let regen_net_list = verilog.gen();
        eprintln!("{}", regen_net_list);
        let regen_verilog = Verilog::from_net_list(regen_net_list);
        assert_eq!(verilog, regen_verilog);
    }
}
