use std::io;

#[derive(Debug)]
struct ShellInput {
    command: String,
    options: Vec<String>,
}

impl ShellInput {
    pub fn exit() -> Self {
        ShellInput {
            command: String::from("exit"),
            options: vec![],
        }
    }
}

fn main() -> io::Result<()> {
    loop {
        if let Ok(input) = parse_std_input() {
            println!("{:?}", input);
            if input.command == "exit" {
                break Ok(());
            }
        }
    }
}

fn parse_std_input() -> io::Result<ShellInput> {
    let mut buffer = String::new();
    let stdin = io::stdin();
    if stdin.read_line(&mut buffer)? == 0 {
        return Ok(ShellInput::exit());
    }
    let mut input = buffer.split_whitespace().map(|s| s.to_string());
    if input.clone().count() == 0 {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    }
    let input = ShellInput {
        command: input.next().unwrap(),
        options: input.collect(),
    };
    Ok(input)
}
