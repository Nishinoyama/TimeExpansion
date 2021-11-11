use std::slice::Iter;
use crate::verilog::ast::token::Token;

enum Module {
    TopModule(),
    OtherModule(),
}

#[derive(Clone, Debug)]
struct Parser<'a> {
    tokens: Iter<'a, &'a Token>,
}

impl<'a> Parser<'a> {
    pub fn from_tokens(tokens: Iter<'a, &'a Token>) -> Self {
        Self { tokens }
    }
}

impl Parser {
    pub fn except_reserved(&mut self, res: &String) -> Result<(), String> {
        if let Some(token) = self.next() {
            if token.name_is(res) {
                Ok(())
            } else {
                Err(format!("Error: Unexpected Token {:?}", token))
            }
        } else {
            Err(format!("Error: Unexpected Token {:?}", token))
        }
    }
    pub fn parse_module(&mut self) -> Module {
    }
    pub fn current(&mut self) -> Option<&Token> {
        *self.tokens.clone().next()
    }
    pub fn next(&mut self) -> Option<&Token> {
        *self.tokens.next()
    }
}
