pub trait NetlistSerializer {
    fn gen(&self) -> String;
    fn multi_gen<T: NetlistSerializer, U: IntoIterator<Item = T> + Clone>(
        iterable: &U,
        joiner: &str,
    ) -> String {
        iterable
            .clone()
            .into_iter()
            .map(|s| s.gen())
            .collect::<Vec<String>>()
            .join(joiner)
    }
}

impl NetlistSerializer for String {
    fn gen(&self) -> String {
        self.clone()
    }
}

#[cfg(test)]
mod test {
    use crate::verilog::netlist_serializer::NetlistSerializer;
    use crate::verilog::Verilog;

    #[test]
    fn expansion_config() {
        let verilog = Verilog::from_file("b15_net.v".to_string()).unwrap();
        let regen_net_list = verilog.gen();
        let regen_verilog = Verilog::from_net_list(regen_net_list);
        assert_eq!(verilog, regen_verilog);
    }
}
