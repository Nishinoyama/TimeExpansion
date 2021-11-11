use std::str::Chars;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Token {
    Reserved(String),
    Identifier(String),
    Number(String),
}

impl Token {
    pub fn to_string(&self) -> String {
        use crate::verilog::ast::token::Token::*;
        match self {
            Reserved(name) | Identifier(name) | Number(name) => name.clone(),
        }
    }
}

#[derive(Debug)]
pub struct Lexer<'a> {
    input: &'a str,
    index: usize,
}

impl<'a> Lexer<'a> {
    pub fn from_str(input: &'a str) -> Self {
        Self { input, index: 0 }
    }
    pub fn from_chars(chars: Chars<'a>) -> Self {
        Self::from_str(chars.as_str())
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
    fn current(&self) -> Option<char> {
        self.get_char(self.index)
    }
    fn current_chars(&self) -> Chars {
        let mut chars = self.input.chars();
        if self.index != 0 {
            chars.nth(self.index - 1);
        };
        chars
    }
    fn next(&mut self) -> Option<char> {
        self.index += 1;
        self.get_char(self.index - 1)
    }
    fn nth(&mut self, n: usize) -> Option<char> {
        self.index += n + 1;
        self.get_char(self.index - 1)
    }
    fn get_char(&self, n: usize) -> Option<char> {
        self.input.chars().nth(n)
    }
    fn get_substr(&self, len: usize) -> Option<&str> {
        self.input.get(self.index..(self.index + len))
    }
    fn is_match(&self, pat: &str) -> bool {
        let n = pat.len();
        let eo_char = self.get_char(self.index + n).unwrap_or(' ');
        self.is_match_without_delimiter(pat) && !eo_char.is_alphanumeric() && eo_char != '_'
    }
    fn is_match_without_delimiter(&self, pat: &str) -> bool {
        let n = pat.len();
        if let Some(substr) = self.get_substr(n) {
            substr.eq(pat)
        } else {
            false
        }
    }
    fn consume_reserved_token(&mut self) -> Option<Token> {
        let reserved = vec!["endmodule", "module", "output", "assign", "input", "wire"];
        reserved
            .into_iter()
            .filter_map(|pat| {
                if self.is_match(pat) {
                    self.nth(pat.len() - 1);
                    Some(Token::Reserved(pat.to_string()))
                } else {
                    None
                }
            })
            .next()
    }
    fn consume_reserved_single_token(&mut self) -> Option<Token> {
        let reserved = "()[]:;,.=+&~^".chars();
        let current = self.current().unwrap();
        reserved
            .into_iter()
            .filter_map(|c| {
                if current == c {
                    self.next();
                    Some(Token::Reserved(c.to_string()))
                } else {
                    None
                }
            })
            .next()
    }
    fn consume_identifier_token(&mut self) -> Option<Token> {
        if !self.current().unwrap().is_alphabetic() {
            None
        } else {
            let tmp_input = self.current_chars();
            let ident_name: String = tmp_input
                .take_while(|c| c.is_alphanumeric() || c.eq(&'_'))
                .collect();
            self.nth(ident_name.len() - 1);
            Some(Token::Identifier(ident_name))
        }
    }
    fn consume_number_token(&mut self) -> Option<Token> {
        if !self.current().unwrap().is_numeric() {
            None
        } else {
            let tmp_input = self.current_chars();
            let number: String = tmp_input
                .take_while(|c| c.is_numeric() || c.eq(&'\''))
                .collect();
            self.nth(number.len() - 1);
            Some(Token::Number(number))
        }
    }
}

#[cfg(test)]
mod test {
    use crate::verilog::ast::token::Lexer;

    #[test]
    fn tokenize() {
        let lexer = Lexer::from_str(
            "module or( a, b, z ); input a, b; output z; assign z = a + b; endmodule",
        );
        eprintln!("{:?}", lexer.tokenize());
    }
}
