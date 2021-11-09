use std::str::Chars;

#[derive(Clone, Debug)]
pub enum Token {
    Reserved(String),
    Identifier(String),
    Number(String),
}

#[derive(Debug)]
pub struct Lexer<'a> {
    input: Chars<'a>,
}

impl<'a> Lexer<'a> {
    pub fn from_chars(chars: Chars<'a>) -> Self {
        Self { input: chars }
    }
}

impl Lexer<'_> {
    pub fn tokenize(mut self) -> Vec<Token> {
        let mut tokens = vec![];
        while let Some(c) = self.current() {
            if c.is_whitespace() {
                self.next();
            } else if let Some(token) = self.consume_reserved_single_token() {
                tokens.push(token);
            } else if let Some(token) = self.consume_reserved_token() {
                tokens.push(token);
            } else if let Some(token) = self.consume_identifier_token() {
                tokens.push(token);
            } else if let Some(token) = self.consume_number_token() {
                tokens.push(token);
            } else {
                self.next();
            }
        }
        tokens
    }
    pub fn token(&mut self) -> Option<Token> {
        None
    }
    pub fn current(&mut self) -> Option<char> {
        self.input.clone().next()
    }
    pub fn next(&mut self) -> Option<char> {
        self.input.next()
    }
    pub fn consume_reserved_token(&mut self) -> Option<Token> {
        let reserved = vec![
            "endmodule",
            "module",
            "output",
            "assign",
            "input",
            "wire",
        ];
        reserved
            .iter()
            .filter_map(|&res| {
                let n = res.len();
                let mut tmp_input = self.input.clone();
                let get_str: String = tmp_input.clone().take(n).collect();
                let eo_char = tmp_input.nth(n).unwrap_or('$');
                if res.eq(get_str.as_str()) && eo_char.is_whitespace() {
                    self.input.nth(n - 1);
                    Some(Token::Reserved(get_str.to_string()))
                } else {
                    None
                }
            })
            .next()
    }
    pub fn consume_reserved_single_token(&mut self) -> Option<Token> {
        let reserved = vec![
            "(",
            ")",
            "[",
            "]",
            ":",
            ";",
            ",",
        ];
        reserved
            .iter()
            .filter_map(|&res| {
                let n = res.len();
                let mut tmp_input = self.input.clone();
                let get_str: String = tmp_input.clone().take(n).collect();
                if res.eq(get_str.as_str()) {
                    self.input.nth(n - 1);
                    Some(Token::Reserved(get_str.to_string()))
                } else {
                    None
                }
            })
            .next()
    }
    pub fn consume_identifier_token(&mut self) -> Option<Token> {
        let mut tmp_input = self.input.clone();
        if !self.current().unwrap().is_alphabetic() {
            None
        } else {
            let ident_name: String = tmp_input
                .take_while(|c| c.is_alphanumeric() || c.eq(&'_'))
                .collect();
            self.input.nth(ident_name.len() - 1);
            Some(Token::Identifier(ident_name))
        }
    }
    pub fn consume_number_token(&mut self) -> Option<Token> {
        let mut tmp_input = self.input.clone();
        if !self.current().unwrap().is_numeric() {
            None
        } else {
            let number: String = tmp_input
                .take_while(|c| c.is_numeric() || c.eq(&'\''))
                .collect();
            self.input.nth(number.len() - 1);
            Some(Token::Number(number))
        }
    }
}

#[cfg(test)]
mod test {
    use crate::verilog::ast::token::Lexer;

    #[test]
    fn tokenize() {
        let lexer = Lexer::from_chars("module or( a, b, z ); input a, b; output z; assign z = a + b; endmodule".chars());
        eprintln!("{:?}", lexer.tokenize());
    }
}
