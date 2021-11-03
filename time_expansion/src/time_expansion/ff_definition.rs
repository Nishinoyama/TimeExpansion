use std::iter::Enumerate;
use std::slice::Iter;
use regex::Regex;

#[derive(Clone, Default, Debug, Eq, PartialEq)]
pub struct FFDefinition {
    pub(crate) name: String,
    pub(crate) data_in: Vec<String>,
    pub(crate) data_out: Vec<String>,
    pub(crate) control: Vec<String>,
}

impl FFDefinition {
    pub fn set_name(&mut self, name: &String) {
        self.name = name.clone();
    }

    pub fn from_file_iter(line_iter: &mut Enumerate<Iter<String>>) -> Self {
        let mut ff_defines = Self::default();
        let data_in_regex = Regex::new(r"\s*data-in\s+(.+)\s*").unwrap();
        let data_out_regex = Regex::new(r"\s*data-out\s+(.+)\s*").unwrap();
        let control_regex = Regex::new(r"\s*control\s+(.+)\s*").unwrap();
        let empty_line_regex = Regex::new(r"^\s*$").unwrap();

        while let Some((mut i, mut ff_line)) = line_iter.next() {
            if ff_line.contains("}") { break; }
            if let Some(cap) = data_in_regex.captures(ff_line) {
                cap.get(1).unwrap().as_str().split(",").for_each(|data|
                    ff_defines.data_in.push(data.trim().to_string())
                );
            } else if let Some(cap) = data_out_regex.captures(ff_line) {
                cap.get(1).unwrap().as_str().split(",").for_each(|data|
                    ff_defines.data_out.push(data.trim().to_string())
                );
            } else if let Some(cap) = control_regex.captures(ff_line) {
                cap.get(1).unwrap().as_str().split(",").for_each(|data|
                    ff_defines.control.push(data.trim().to_string())
                );
            } else if empty_line_regex.is_match(ff_line) {
            } else {
                eprintln!("Error: Undefined FF Option");
                eprintln!("Syntax error at line {}", i+1);
                eprintln!("{}", ff_line);
                panic!("Syntax Error at line {}", i+1);
            }
        };
        return ff_defines;
    }
}
