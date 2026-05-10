#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Keyword(Keyword),
    Identifier(String),
    Number(String),
    String(String),
    Op(String),
    LParen, RParen,
    LBrace, RBrace,
    Comma, Dot, Colon,
    Newline,
    Unknown(char),
    EOF,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TokenData {
    pub token: Token,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Keyword {
    Six, End, Fn, V, If, Else, For, Ret, Use, Try, Put, Get, Ptr, Leak, Report, As,
}

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }
    pub fn tokenize(&mut self) -> Vec<TokenData> {
        let mut tokens = Vec::new();
        loop {
            let td = self.next_token();
            let is_eof = td.token == Token::EOF;
            tokens.push(td);
            if is_eof { break; }
        }
        tokens
    }

    pub fn next_token(&mut self) -> TokenData {
        let is_newline = self.skip_whitespace();
        if is_newline {
            return TokenData { token: Token::Newline, line: self.line - 1, col: self.col };
        }

        let current_line = self.line;
        let current_col = self.col;

        if self.pos >= self.input.len() {
            return TokenData { token: Token::EOF, line: current_line, col: current_col };
        }

        let ch = self.input[self.pos];

        let token = if ch.is_alphabetic() || ch == '_' {
            self.read_identifier()
        } else if ch.is_numeric() {
            self.read_number()
        } else if ch == '"' {
            self.read_string()
        } else {
            self.pos += 1;
            self.col += 1;
            match ch {
                '(' => Token::LParen,
                ')' => Token::RParen,
                '{' => Token::LBrace,
                '}' => Token::RBrace,
                ',' => Token::Comma,
                '.' => {
                    if self.pos < self.input.len() && self.input[self.pos] == '.' {
                        self.pos += 1;
                        self.col += 1;
                        Token::Op("..".to_string())
                    } else {
                        Token::Dot
                    }
                }
                ':' => Token::Colon,
                '&' => Token::Op("&".to_string()),
                '#' => Token::Op("#".to_string()),
                '@' => Token::Op("@".to_string()),
                '+' | '-' | '*' | '/' | '=' | '<' | '>' | '!' | '^' | '|' => {
                    let mut op = ch.to_string();
                    if self.pos < self.input.len() {
                        let next = self.input[self.pos];
                        if next == '=' {
                            op.push(next);
                            self.pos += 1;
                            self.col += 1;
                        }
                    }
                    Token::Op(op)
                }
                _ => Token::Unknown(ch),
            }
        };

        TokenData { token, line: current_line, col: current_col }
    }

    fn read_identifier(&mut self) -> Token {
        let start = self.pos;
        while self.pos < self.input.len() && (self.input[self.pos].is_alphanumeric() || self.input[self.pos] == '_') {
            self.pos += 1;
            self.col += 1;
        }
        let text: String = self.input[start..self.pos].iter().collect();
        match text.as_str() {
            "six" => Token::Keyword(Keyword::Six),
            "end" => Token::Keyword(Keyword::End),
            "fn" => Token::Keyword(Keyword::Fn),
            "v" => Token::Keyword(Keyword::V),
            "if" => Token::Keyword(Keyword::If),
            "else" => Token::Keyword(Keyword::Else),
            "for" => Token::Keyword(Keyword::For),
            "ret" => Token::Keyword(Keyword::Ret),
            "use" => Token::Keyword(Keyword::Use),
            "try" => Token::Keyword(Keyword::Try),
            "put" => Token::Keyword(Keyword::Put),
            "get" => Token::Keyword(Keyword::Get),
            "ptr" => Token::Keyword(Keyword::Ptr),
            "leak" => Token::Keyword(Keyword::Leak),
            "report" => Token::Keyword(Keyword::Report),
            "as" => Token::Keyword(Keyword::As),
            _ => Token::Identifier(text),
        }
    }

    fn read_number(&mut self) -> Token {
        let start = self.pos;
        while self.pos < self.input.len() && self.input[self.pos].is_numeric() {
            self.pos += 1;
            self.col += 1;
        }
        Token::Number(self.input[start..self.pos].iter().collect())
    }

    fn read_string(&mut self) -> Token {
        self.pos += 1;
        self.col += 1;
        let start = self.pos;
        while self.pos < self.input.len() && self.input[self.pos] != '"' {
            self.pos += 1;
            self.col += 1;
        }
        let text = self.input[start..self.pos].iter().collect();
        if self.pos < self.input.len() {
            self.pos += 1;
            self.col += 1;
        }
        Token::String(text)
    }

    fn skip_whitespace(&mut self) -> bool {
        let mut newline = false;
        while self.pos < self.input.len() {
            let ch = self.input[self.pos];
            if ch.is_whitespace() {
                if ch == '\n' {
                    self.line += 1;
                    self.col = 1;
                    newline = true;
                } else {
                    self.col += 1;
                }
                self.pos += 1;
            } else if ch == '/' && self.pos + 1 < self.input.len() && self.input[self.pos + 1] == '/' {
                // Skip comment
                while self.pos < self.input.len() && self.input[self.pos] != '\n' {
                    self.pos += 1;
                    self.col += 1;
                }
                // pos is now at \n or EOF. The next iteration of outer loop will handle it.
            } else {
                break;
            }
        }
        newline
    }
}
