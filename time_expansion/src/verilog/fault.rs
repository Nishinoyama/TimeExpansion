/// WIP! transition str/stf fault only!
#[derive(Debug)]
pub struct Fault {
    location: String,
    sa_value: bool,
}

impl Fault {
    pub fn new(location: String, sa_value: bool) -> Self {
        Self { location, sa_value }
    }
    pub fn location(&self) -> &str {
        &self.location
    }
    pub fn sa_value(&self) -> bool {
        self.sa_value
    }
}
