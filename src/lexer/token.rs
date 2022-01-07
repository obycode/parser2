#[derive(Debug)]
pub enum Token {
    Eof,
    Whitespace,
    Lparen,
    Rparen,
    Lbrace,
    Rbrace,
    Colon,
    Comma,
    Dot,
    Int(i128),
    Uint(u128),
    Buff(Vec<char>),
    Ident(Vec<char>),
    Plus,
    Minus,
    Multiply,
    Divide,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Comment(Vec<char>),
    Invalid,
}

pub struct Span {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

pub struct PlacedToken {
    pub span: Span,
    pub token: Token,
}