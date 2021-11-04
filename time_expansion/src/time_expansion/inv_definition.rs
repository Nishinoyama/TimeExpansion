use std::iter::Enumerate;
use std::slice::Iter;
use regex::Regex;

#[derive(Clone, Default, Debug, Eq, PartialEq)]
pub struct InvDefinition {
    pub(crate) name: String,
    pub(crate) input: String,
    pub(crate) output: String,
}

impl InvDefinition {
    pub fn set_name(&mut self, name: &String) {
        self.name = name.clone();
    }

    pub fn is_empty(&self) -> bool {
        self.name.is_empty() || self.input.is_empty() || self.output.is_empty()
    }

    pub fn from_file_iter(line_iter: &mut Enumerate<Iter<String>>) -> Self {
        let mut inv_defines = Self::default();
        let input_regex = Regex::new(r"\s*input\s+(\w+)\s*").unwrap();
        let output_regex = Regex::new(r"\s*output\s+(\w+)\s*").unwrap();
        let empty_line_regex = Regex::new(r"^\s*$").unwrap();

        while let Some((mut i, mut inv_line)) = line_iter.next() {
            if inv_line.contains("}") { break; }
            if let Some(cap) = input_regex.captures(inv_line) {
                inv_defines.input = cap.get(1).unwrap().as_str().trim().to_string();
            } else if let Some(cap) = output_regex.captures(inv_line) {
                inv_defines.output = cap.get(1).unwrap().as_str().trim().to_string();
            } else if empty_line_regex.is_match(inv_line) {
            } else {
                eprintln!("Error: Undefined Inv Option");
                eprintln!("Syntax error at line {}", i+1);
                eprintln!("{}", inv_line);
                panic!("Syntax Error at line {}", i+1);
            }
        };
        return inv_defines;
    }
}
