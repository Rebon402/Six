use crate::lexer::{Keyword, Token, TokenData};

#[derive(Debug, Clone)]
pub enum Expr {
    Number(String),
    String(String),
    Variable(String),
    Addr(String),
    Deref(Box<Expr>),
    Cast(Box<Expr>, String),
    BinaryOp(Box<Expr>, String, Box<Expr>),
    Call(String, Vec<Expr>),
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Six(String, Vec<Stmt>),
    VarDecl(String, Option<String>, Expr),
    #[allow(dead_code)]
    Assignment(String, Expr),
    DerefAssignment(Expr, Expr),
    FnDecl(String, Vec<String>, Vec<Stmt>),
    #[allow(dead_code)]
    If(Expr, Vec<Stmt>, Option<Vec<Stmt>>),
    For(String, Expr, Expr, Vec<Stmt>),
    Return(Expr),
    #[allow(dead_code)]
    Use(String),
    Try(Vec<Stmt>),
    Leak,
    Report,
    Directive(String, Vec<Expr>),
    Put(Expr),
    #[allow(dead_code)]
    Get(String),
    Expression(Expr),
}

pub struct Parser {
    tokens: Vec<TokenData>,
    pos: usize,
    source: String,
}

impl Parser {
    pub fn new(tokens: Vec<TokenData>, source: String) -> Self {
        Self {
            tokens,
            pos: 0,
            source,
        }
    }

    fn report_error(&self, message: &str, line: usize, col: usize) -> ! {
        let lines: Vec<&str> = self.source.lines().collect();
        let display_line = if line > 0 { line } else { 1 };
        let source_line = lines.get(display_line - 1).unwrap_or(&"");

        eprintln!("\n[SixC ERROR] {}", message);
        eprintln!("  --> line {}:{}", line, col);
        eprintln!("   |");
        eprintln!("{:>3} | {}", display_line, source_line);
        eprintln!(
            "   | {}{}",
            " ".repeat(if col > 0 { col - 1 } else { 0 }),
            "^"
        );
        eprintln!("   |");
        std::process::exit(1);
    }

    pub fn parse(&mut self) -> Vec<Stmt> {
        let mut statements = Vec::new();
        while !self.is_at_end() {
            while self.check(Token::Newline) {
                self.advance();
            }
            if self.is_at_end() {
                break;
            }

            if self.check_keyword(Keyword::Six) {
                if let Some(s) = self.parse_six() {
                    statements.push(s);
                }
            } else {
                let data = self.peek_data();
                self.report_error(
                    "Every .six project must start with 'six <name>'",
                    data.line,
                    data.col,
                );
            }
        }

        let has_main = statements.iter().any(|s| {
            if let Stmt::Six(_, body) = s {
                body.iter()
                    .any(|bs| matches!(bs, Stmt::FnDecl(name, _, _) if name == "main"))
            } else {
                false
            }
        });

        if !has_main {
            eprintln!("\n[SixC ERROR] Missing 'fn main()' in the root 'six' block.");
            std::process::exit(1);
        }

        statements
    }

    fn parse_six(&mut self) -> Option<Stmt> {
        self.advance(); // six
        let data = self.peek_data();
        if let Token::Identifier(name) = self.advance() {
            let mut body = Vec::new();
            while !self.check_keyword(Keyword::End) && !self.is_at_end() {
                if let Some(s) = self.parse_statement() {
                    body.push(s);
                }
            }
            self.consume_keyword(Keyword::End);
            Some(Stmt::Six(name, body))
        } else {
            self.report_error("Expected project name after 'six'", data.line, data.col);
        }
    }

    fn parse_statement(&mut self) -> Option<Stmt> {
        while self.check(Token::Newline) {
            self.advance();
        }
        if self.is_at_end() {
            return None;
        }
        if self.check_keyword(Keyword::End) {
            return None;
        }

        let data = self.peek_data();
        match data.token {
            Token::Keyword(Keyword::Six) => self.parse_six(),
            Token::Keyword(Keyword::V) => self.parse_var_decl(),
            Token::Keyword(Keyword::Fn) => self.parse_fn_decl(),
            Token::Keyword(Keyword::If) => self.parse_if(),
            Token::Keyword(Keyword::For) => self.parse_for(),
            Token::Keyword(Keyword::Ret) => self.parse_return(),
            Token::Keyword(Keyword::Use) => self.parse_use(),
            Token::Keyword(Keyword::Try) => self.parse_try(),
            Token::Keyword(Keyword::Put) => self.parse_put(),
            Token::Keyword(Keyword::Get) => self.parse_get(),
            Token::Keyword(Keyword::Leak) => {
                self.advance();
                Some(Stmt::Leak)
            }
            Token::Keyword(Keyword::Report) => {
                self.advance();
                Some(Stmt::Report)
            }
            Token::Identifier(_) => self.parse_identifier_stmt(),
            Token::Op(ref op) if op == "*" || op == "@" => self.parse_deref_assignment(),
            Token::Op(ref op) if op == "#" => self.parse_directive(),
            _ => self.parse_expression_stmt(),
        }
    }

    fn parse_identifier_stmt(&mut self) -> Option<Stmt> {
        let _name_data = self.peek_data();
        if let Token::Identifier(name) = self.advance() {
            if let Token::Op(op) = self.peek() {
                if op == "=" {
                    self.advance(); // =
                    let value = self.parse_expression();
                    return Some(Stmt::Assignment(name, value));
                }
            }
            // If not assignment, backtrack or handle as expression
            // Since we already advanced, we create a Variable expr and then Expression stmt
            Some(Stmt::Expression(Expr::Call(name, self.parse_call_args())))
        } else {
            None
        }
    }

    fn parse_call_args(&mut self) -> Vec<Expr> {
        let mut args = Vec::new();
        if self.check(Token::LParen) {
            self.advance();
            while !self.check(Token::RParen) {
                args.push(self.parse_expression());
                if self.check(Token::Comma) {
                    self.advance();
                }
            }
            self.consume(Token::RParen);
        }
        args
    }

    fn parse_var_decl(&mut self) -> Option<Stmt> {
        self.advance(); // v
        let data = self.peek_data();
        if let Token::Identifier(name) = data.token {
            self.advance();

            let mut var_type = None;
            if self.check(Token::Colon) {
                self.advance(); // :
                let type_data = self.peek_data();
                if let Token::Identifier(t) = self.advance() {
                    var_type = Some(t);
                } else {
                    self.report_error(
                        "Expected type name after ':'",
                        type_data.line,
                        type_data.col,
                    );
                }
            }

            self.consume_op("=");
            let value = self.parse_expression();
            Some(Stmt::VarDecl(name, var_type, value))
        } else {
            self.report_error("Expected identifier after 'v'", data.line, data.col);
        }
    }

    fn parse_fn_decl(&mut self) -> Option<Stmt> {
        self.advance(); // fn
        let data = self.peek_data();
        if let Token::Identifier(name) = data.token {
            self.advance();
            self.consume(Token::LParen);
            let mut params = Vec::new();
            while !self.check(Token::RParen) {
                let p_data = self.peek_data();
                if let Token::Identifier(p) = self.advance() {
                    if self.check(Token::Colon) {
                        self.advance();
                        self.advance();
                    }
                    params.push(p);
                } else {
                    self.report_error("Expected parameter name", p_data.line, p_data.col);
                }
                if self.check(Token::Comma) {
                    self.advance();
                }
            }
            self.consume(Token::RParen);

            let mut body = Vec::new();
            while !self.check_keyword(Keyword::End) && !self.is_at_end() {
                if let Some(s) = self.parse_statement() {
                    body.push(s);
                }
            }
            self.consume_keyword(Keyword::End);
            Some(Stmt::FnDecl(name, params, body))
        } else {
            self.report_error("Expected function name", data.line, data.col);
        }
    }

    fn parse_if(&mut self) -> Option<Stmt> {
        self.advance(); // if
        let condition = self.parse_expression();
        let mut then_branch = Vec::new();
        while !self.check_keyword(Keyword::End)
            && !self.check_keyword(Keyword::Else)
            && !self.is_at_end()
        {
            if let Some(s) = self.parse_statement() {
                then_branch.push(s);
            }
        }

        let mut else_branch = None;
        if self.check_keyword(Keyword::Else) {
            self.advance();
            let mut eb = Vec::new();
            while !self.check_keyword(Keyword::End) && !self.is_at_end() {
                if let Some(s) = self.parse_statement() {
                    eb.push(s);
                }
            }
            else_branch = Some(eb);
        }

        self.consume_keyword(Keyword::End);
        Some(Stmt::If(condition, then_branch, else_branch))
    }

    fn parse_for(&mut self) -> Option<Stmt> {
        self.advance(); // for
        let data = self.peek_data();
        if let Token::Identifier(var) = self.advance() {
            self.consume_op("=");
            let start = self.parse_expression();
            self.consume_op("..");
            let end = self.parse_expression();

            let mut body = Vec::new();
            while !self.check_keyword(Keyword::End) && !self.is_at_end() {
                if let Some(s) = self.parse_statement() {
                    body.push(s);
                }
            }
            self.consume_keyword(Keyword::End);
            Some(Stmt::For(var, start, end, body))
        } else {
            self.report_error("Expected identifier for loop variable", data.line, data.col);
        }
    }

    fn parse_return(&mut self) -> Option<Stmt> {
        self.advance();
        Some(Stmt::Return(self.parse_expression()))
    }

    fn parse_use(&mut self) -> Option<Stmt> {
        self.advance();
        if let Token::String(s) = self.advance() {
            Some(Stmt::Use(s))
        } else {
            let data = self.peek_data();
            self.report_error("Expected string after 'use'", data.line, data.col);
        }
    }

    fn parse_try(&mut self) -> Option<Stmt> {
        self.advance();
        let mut body = Vec::new();
        while !self.check_keyword(Keyword::End) && !self.is_at_end() {
            if let Some(s) = self.parse_statement() {
                body.push(s);
            }
        }
        self.consume_keyword(Keyword::End);
        Some(Stmt::Try(body))
    }

    fn parse_put(&mut self) -> Option<Stmt> {
        self.advance();
        Some(Stmt::Put(self.parse_expression()))
    }

    fn parse_get(&mut self) -> Option<Stmt> {
        self.advance();
        if let Token::Identifier(id) = self.advance() {
            Some(Stmt::Get(id))
        } else {
            let data = self.peek_data();
            self.report_error("Expected identifier after 'get'", data.line, data.col);
        }
    }

    fn parse_expression_stmt(&mut self) -> Option<Stmt> {
        Some(Stmt::Expression(self.parse_expression()))
    }

    fn parse_expression(&mut self) -> Expr {
        self.parse_binary()
    }

    fn parse_binary(&mut self) -> Expr {
        let mut left = self.parse_cast();
        while !self.is_at_end() {
            let next = self.peek();
            if next == Token::Newline {
                break;
            }
            if let Token::Op(op) = next {
                if op == ".." || op == "&" || op == "@" || op == "=" {
                    break;
                }
                self.advance();
                let right = self.parse_cast();
                left = Expr::BinaryOp(Box::new(left), op, Box::new(right));
            } else {
                break;
            }
        }
        left
    }

    fn parse_cast(&mut self) -> Expr {
        let mut expr = self.parse_primary();
        while self.check_keyword(Keyword::As) {
            self.advance(); // as
            let type_data = self.peek_data();
            if let Token::Identifier(t) = self.advance() {
                expr = Expr::Cast(Box::new(expr), t);
            } else {
                self.report_error(
                    "Expected type name after 'as'",
                    type_data.line,
                    type_data.col,
                );
            }
        }
        expr
    }

    fn parse_primary(&mut self) -> Expr {
        let data = self.peek_data();
        match self.advance() {
            Token::Op(ref op) if op == "&" => {
                let id_data = self.peek_data();
                if let Token::Identifier(id) = self.advance() {
                    Expr::Addr(id)
                } else {
                    self.report_error("Expected identifier after '&'", id_data.line, id_data.col);
                }
            }
            Token::Op(ref op) if op == "*" => Expr::Deref(Box::new(self.parse_primary())),
            Token::Op(ref op) if op == "@" => Expr::Deref(Box::new(self.parse_primary())),
            Token::Number(n) => Expr::Number(n),
            Token::String(s) => Expr::String(s),
            Token::Identifier(id) => {
                if self.check(Token::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    while !self.check(Token::RParen) {
                        args.push(self.parse_expression());
                        if self.check(Token::Comma) {
                            self.advance();
                        }
                    }
                    self.consume(Token::RParen);
                    Expr::Call(id, args)
                } else {
                    Expr::Variable(id)
                }
            }
            Token::LParen => {
                let expr = self.parse_expression();
                self.consume(Token::RParen);
                expr
            }
            _ => self.report_error("Unexpected token", data.line, data.col),
        }
    }

    fn parse_directive(&mut self) -> Option<Stmt> {
        self.advance(); // skip #
        if let Token::Identifier(name) = self.advance() {
            let mut args = Vec::new();
            if name == "const" {
                args.push(self.parse_expression());
                self.consume_op("=");
                args.push(self.parse_expression());
            }
            Some(Stmt::Directive(name, args))
        } else {
            let data = self.peek_data();
            self.report_error("Expected identifier after '#'", data.line, data.col);
        }
    }

    fn parse_deref_assignment(&mut self) -> Option<Stmt> {
        self.advance(); // skip * or @
        let addr = self.parse_expression();
        self.consume_op("=");
        let value = self.parse_expression();
        Some(Stmt::DerefAssignment(addr, value))
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len() || self.tokens[self.pos].token == Token::EOF
    }

    fn advance(&mut self) -> Token {
        if self.is_at_end() {
            Token::EOF
        } else {
            self.pos += 1;
            self.tokens[self.pos - 1].token.clone()
        }
    }

    fn peek(&self) -> Token {
        if self.is_at_end() {
            Token::EOF
        } else {
            self.tokens[self.pos].token.clone()
        }
    }

    fn peek_data(&self) -> TokenData {
        if self.is_at_end() {
            TokenData {
                token: Token::EOF,
                line: 0,
                col: 0,
            }
        } else {
            self.tokens[self.pos].clone()
        }
    }

    fn check(&self, token: Token) -> bool {
        self.peek() == token
    }

    fn check_keyword(&self, kw: Keyword) -> bool {
        if let Token::Keyword(k) = self.peek() {
            return k == kw;
        }
        false
    }

    fn consume(&mut self, token: Token) {
        if self.check(token.clone()) {
            self.advance();
        } else {
            let data = self.peek_data();
            self.report_error(&format!("Expected {:?}", token), data.line, data.col);
        }
    }

    fn consume_keyword(&mut self, kw: Keyword) {
        if self.check_keyword(kw.clone()) {
            self.advance();
        } else {
            let data = self.peek_data();
            self.report_error(&format!("Expected {:?}", kw), data.line, data.col);
        }
    }

    fn consume_op(&mut self, op: &str) {
        if let Token::Op(ref o) = self.peek() {
            if o == op {
                self.advance();
                return;
            }
        }
        let data = self.peek_data();
        self.report_error(&format!("Expected operator '{}'", op), data.line, data.col);
    }
}
