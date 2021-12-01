use crate::verilog::ast::token::Token;
use crate::verilog::{Gate, Module, PortWire, SignalRange, Verilog, Wire};

#[derive(Clone, Debug)]
pub struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    /// Generates `Parser` with [`Vec`] of [`Token`]s
    pub fn from_tokens(tokens: Vec<Token>) -> Self {
        Self {
            tokens: tokens.into_iter().rev().collect(),
            index: 0,
        }
    }
    fn consume_token_if_eq(&mut self, expected_token: Token) -> Result<Option<Token>, ParseError> {
        Ok(if let Some(token) = self.current() {
            if expected_token.eq(token) {
                self.next()
            } else {
                None
            }
        } else {
            None
        })
    }
    fn consume_identifier_token(&mut self) -> Result<Option<Token>, ParseError> {
        Ok(if let Some(token) = self.current() {
            if let Token::Identifier(_) = token {
                self.next()
            } else {
                None
            }
        } else {
            None
        })
    }
    fn consume_reserved_token(&mut self, name: &str) -> Result<Option<Token>, ParseError> {
        let expected_token = Token::Reserved(name.to_string());
        self.consume_token_if_eq(expected_token)
    }
    fn consume_number_token(&mut self) -> Result<Option<Token>, ParseError> {
        Ok(if let Some(token) = self.current() {
            if let Token::Number(_) = token {
                self.next()
            } else {
                None
            }
        } else {
            None
        })
    }
    fn expect_token(&mut self) -> Result<Token, ParseError> {
        if let Some(token) = self.next() {
            Ok(token)
        } else {
            Err(ParseError::from(ParseErrorType::NoToken))
        }
    }
    fn expect_reserved_token(&mut self, name: &str) -> Result<Token, ParseError> {
        let expected_token = Token::Reserved(name.to_string());
        let token = self.expect_token()?;
        if token == expected_token {
            Ok(token)
        } else {
            Err(ParseError::from(ParseErrorType::UnexpectedToken(
                expected_token,
                token,
            )))
        }
    }
    fn expect_number(&mut self) -> Result<Token, ParseError> {
        let token = self.expect_token()?;
        if matches!(token, Token::Number(_)) {
            Ok(token)
        } else {
            Err(ParseError::from(ParseErrorType::UnexpectedToken(
                Token::Number(String::from("{number}")),
                token,
            )))
        }
    }
    fn expect_identifier(&mut self) -> Result<Token, ParseError> {
        let token = self.expect_token()?;
        if matches!(token, Token::Identifier(_)) {
            Ok(token)
        } else {
            Err(ParseError::from(ParseErrorType::UnexpectedToken(
                Token::Identifier(String::from("{identifier}")),
                token,
            )))
        }
    }
    /// Generates [`Verilog`] netlist.
    ///
    /// ```ebnf
    /// verilog := module*
    /// ```
    pub fn verilog(mut self) -> Result<Verilog, ParseError> {
        let mut verilog = Verilog::default();
        while let Some(module) = self.module()? {
            verilog.push_module(module);
        }
        Ok(verilog)
    }
    /// ```ebnf
    /// module := "module" identifier "(" declarations ")" ";" statements* "endmodule"
    /// ```
    fn module(&mut self) -> Result<Option<Module>, ParseError> {
        if let Some(_) = self.consume_reserved_token("module")? {
            let mut module = Module::default();
            *module.name_mut() = self.expect_identifier()?.to_string();
            self.expect_reserved_token("(")?;
            self.declarations(None)?;
            self.expect_reserved_token(")")?;
            self.expect_reserved_token(";")?;
            while self.statement(&mut module)?.is_some() {}
            self.expect_reserved_token("endmodule")?;
            Ok(Some(module))
        } else {
            Ok(None)
        }
    }
    /// ```ebnf
    /// statement := ( (input|output|wire) range? declarations |
    ///                assign expressions |
    ///                identifier identifier "(" gate_ports ")" ) ";"
    /// ```
    fn statement(&mut self, module: &mut Module) -> Result<Option<()>, ParseError> {
        if let Some(_) = self.consume_reserved_token("input")? {
            let range = self.range()?;
            let (range, signals) = self.declarations(range)?;
            signals.into_iter().for_each(|s| {
                module.push_input(Wire::new(range.clone(), s));
            });
        } else if let Some(_) = self.consume_reserved_token("output")? {
            let range = self.range()?;
            let (range, signals) = self.declarations(range)?;
            signals.into_iter().for_each(|s| {
                module.push_output(Wire::new(range.clone(), s));
            });
        } else if let Some(_) = self.consume_reserved_token("wire")? {
            let range = self.range()?;
            let (range, signals) = self.declarations(range)?;
            signals.into_iter().for_each(|s| {
                module.push_wire(Wire::new(range.clone(), s));
            });
        } else if let Some(_) = self.consume_reserved_token("assign")? {
            self.expressions()?
                .into_iter()
                .for_each(|s| module.push_assign(s));
        } else if let Some(gate_name) = self.consume_identifier_token()? {
            let mut gate = Gate::default();
            *gate.name_mut() = gate_name.to_string();
            let ident = self.expect_identifier()?.to_string();
            self.expect_reserved_token("(")?;
            module.push_gate(ident, self.gate_ports(gate)?);
            self.expect_reserved_token(")")?;
        } else {
            return Ok(None);
        }
        self.expect_reserved_token(";")?;
        Ok(Some(()))
    }
    /// ```ebnf
    /// declarations := identifier ( "," identifier )*
    /// ```
    fn declarations(
        &mut self,
        range: Option<(String, String)>,
    ) -> Result<(SignalRange, Vec<String>), ParseError> {
        use SignalRange::*;
        let signal_range = if let Some(sr) = range {
            Multiple(sr.clone())
        } else {
            Single
        };
        let mut declarations = vec![];
        while let Some(token) = self.consume_identifier_token()? {
            declarations.push(token.to_string());
            if self.consume_reserved_token(",")?.is_none() {
                break;
            }
        }
        Ok((signal_range, declarations))
    }
    /// ```ebnf
    /// expressions := expression ( "," expression )*
    /// ```
    fn expressions(&mut self) -> Result<Vec<String>, ParseError> {
        let mut expressions = vec![];
        while let Some(expr) = self.expression()? {
            expressions.push(expr);
            if self.consume_reserved_token(",")?.is_none() {
                break;
            }
        }
        Ok(expressions)
    }
    /// ```ebnf
    /// expression := identifier_range "=" ( [^","";"] )
    /// ```
    fn expression(&mut self) -> Result<Option<String>, ParseError> {
        let lhd = self.identifier_range()?;
        let mut expression = vec![lhd];
        expression.push(self.expect_reserved_token("=")?.to_string());
        loop {
            if let Some(token) = self.current() {
                if token.to_string().eq(",") || token.to_string().eq(";") {
                    break;
                }
            }
            expression.push(self.expect_token()?.to_string());
        }
        Ok(Some(expression.join(" ")))
    }
    /// ```ebnf
    /// range := "[" number ":" number "]"
    /// ```
    fn range(&mut self) -> Result<Option<(String, String)>, ParseError> {
        Ok(if self.consume_reserved_token("[")?.is_some() {
            let r = self.expect_number()?.to_string();
            self.expect_reserved_token(":")?;
            let l = self.expect_number()?.to_string();
            self.expect_reserved_token("]")?;
            Some((r, l))
        } else {
            None
        })
    }
    /// ```ebnf
    /// identifier_range := identifier ( "[" number "]" )?
    /// ```
    fn identifier_range(&mut self) -> Result<String, ParseError> {
        let mut identifier_range = vec![self.expect_identifier()?];
        if let Some(lp) = self.consume_reserved_token("[")? {
            identifier_range.push(lp);
            identifier_range.push(self.expect_number()?);
            identifier_range.push(self.expect_reserved_token("]")?);
        }
        Ok(identifier_range
            .into_iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
            .join(""))
    }
    /// ```ebnf
    /// gate_ports := ( "." identifier "(" identifier_range | number ")" )*
    /// ```
    fn gate_ports(&mut self, mut gate: Gate) -> Result<Gate, ParseError> {
        while let Some(_) = self.consume_reserved_token(".")? {
            let port = self.expect_identifier()?.to_string();
            self.expect_reserved_token("(")?;
            if let Some(wire) = self.consume_number_token()? {
                gate.push_port(PortWire::Constant(port, wire.to_string()));
            } else {
                let wire = self.identifier_range()?.to_string();
                gate.push_port(PortWire::Wire(port, wire));
            }
            self.expect_reserved_token(")")?;
            if self.consume_reserved_token(",")?.is_none() {
                break;
            }
        }
        Ok(gate)
    }
    fn current(&self) -> Option<&Token> {
        self.tokens.last()
    }
    fn next(&mut self) -> Option<Token> {
        self.tokens.pop()
    }
}

#[derive(Debug)]
pub struct ParseError {
    error_type: ParseErrorType,
}

impl From<ParseErrorType> for ParseError {
    fn from(error_type: ParseErrorType) -> Self {
        Self { error_type }
    }
}

#[derive(Debug)]
pub enum ParseErrorType {
    NoToken,
    /// .1 [`Token`] is expected, but .2 [`Token`] gained.
    UnexpectedToken(Token, Token),
}

#[cfg(test)]
mod test {
    use crate::verilog::ast::parser::Parser;
    use crate::verilog::ast::token::Lexer;

    #[test]
    fn parse() {
        let lexer = Lexer::from_str(
            "module or( a, b, z ); input [1:0] a, b; output [1:0] z; assign z[0] = a[0] + b[0]; and u1(.a(a[1]), .b(1'b1), .z(z[1])); endmodule ",
        );
        Parser::from_tokens(lexer.tokenize()).verilog().ok();
    }
}
