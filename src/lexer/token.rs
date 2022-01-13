use super::error::LexerError;

#[derive(Debug, PartialEq)]
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
    AsciiString(String),
    Utf8String(String),
    Bytes(Vec<u8>),
    Ident(String),
    Plus,
    Minus,
    Multiply,
    Divide,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Comment(String),
    Error(LexerError),
    Placeholder, // used to continue parsing after errors
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
