use std::fmt::Formatter;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Token {
    Reserved(String),
    Identifier(String),
    Number(String),
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use crate::verilog::ast::token::Token::*;
        write!(
            f,
            "{}",
            match self {
                Reserved(name) | Identifier(name) | Number(name) => name.clone(),
            }
        )
    }
}

#[derive(Debug)]
pub struct Lexer<'a> {
    input: &'a [u8],
    index: usize,
}

impl<'a> Lexer<'a> {
    /// Generate `Lexer` from [`str`] . Parameter containing non-ascii characters cannot be guaranteed
    /// because `Lexer` treats str as [`u8`] slice ( known as [`str::as_bytes`] ).
    pub fn from_str(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            index: 0,
        }
    }
}

impl Lexer<'_> {
    /// Generate [`Vec`] of [`Token`]s by tokenizing Verilog-netlist source.
    pub fn tokenize(mut self) -> Vec<Token> {
        let mut tokens = vec![];
        while let Some(c) = self.current() {
            if c.is_ascii_whitespace() {
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
    fn current(&self) -> Option<&u8> {
        self.get_char(self.index)
    }
    fn next(&mut self) -> Option<&u8> {
        self.index += 1;
        self.get_char(self.index - 1)
    }
    fn nth(&mut self, n: usize) -> Option<&u8> {
        self.index += n + 1;
        self.get_char(self.index - 1)
    }
    fn get_char(&self, n: usize) -> Option<&u8> {
        self.input.get(n)
    }
    fn get_substr(&self, len: usize) -> Option<&[u8]> {
        self.input.get(self.index..(self.index + len))
    }
    fn is_match(&self, pat: &str) -> bool {
        let n = pat.len();
        let eo_char = *self.get_char(self.index + n).unwrap_or(&(b' '));
        self.is_match_without_delimiter(pat) && !eo_char.is_ascii_alphanumeric() && eo_char != b'_'
    }
    fn is_match_without_delimiter(&self, pat: &str) -> bool {
        let n = pat.len();
        if let Some(substr) = self.get_substr(n) {
            substr.eq(pat.as_bytes())
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
        let current = *self.current().unwrap() as char;
        reserved
            .into_iter()
            .filter_map(|c: char| {
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
        let head = *self.current().unwrap() as char;
        if !head.is_ascii_alphabetic() {
            None
        } else {
            let mut ident_name = String::new();
            while let Some(&c) = self.current() {
                if c.is_ascii_alphanumeric() || c.eq(&b'_') {
                    self.next();
                    ident_name.push(c as char);
                } else {
                    break;
                }
            }
            Some(Token::Identifier(ident_name))
        }
    }
    fn consume_number_token(&mut self) -> Option<Token> {
        let head = *self.current().unwrap() as char;
        if !head.is_ascii_digit() {
            None
        } else {
            let mut number = String::new();
            while let Some(&c) = self.current() {
                if c.is_ascii_digit() || c.eq(&b'\'') {
                    self.next();
                    number.push(c as char);
                } else {
                    break;
                }
            }
            Some(Token::Number(number))
        }
    }
}

#[cfg(test)]
mod test {
    use crate::verilog::ast::token::Lexer;

    #[test]
    fn tokenize() {
        Lexer::from_str("module or( a, b, z ); input a, b; output z; assign z = a + b; endmodule");
    }
}
