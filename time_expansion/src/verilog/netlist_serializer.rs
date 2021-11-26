pub trait NetlistSerializer {
    /// Generates [`Verilog`](crate::verilog::Verilog) netlist source.
    fn gen(&self) -> String;
    /// Generates [`NetlistSerializer`]s' netlist source [joined](slice::join) with `separator`.
    fn multi_gen<T: NetlistSerializer, U: IntoIterator<Item = T> + Clone>(
        iterable: &U,
        separator: &str,
    ) -> String {
        iterable
            .clone()
            .into_iter()
            .map(|s| s.gen())
            .collect::<Vec<String>>()
            .join(separator)
    }
}

impl NetlistSerializer for String {
    /// Returns just [`String`] [clone](String::clone).
    fn gen(&self) -> String {
        self.clone()
    }
}

#[cfg(test)]
mod test {
    use crate::verilog::netlist_serializer::NetlistSerializer;
    use crate::verilog::{Verilog, VerilogError};

    #[test]
    fn expansion_config() -> Result<(), VerilogError> {
        let verilog = Verilog::from_file("b15_net.v".to_string())?;
        let regen_net_list = verilog.gen();
        let regen_verilog = Verilog::from_net_list(regen_net_list)?;
        assert_eq!(verilog, regen_verilog);
        Ok(())
    }
}
