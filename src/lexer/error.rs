#[derive(Debug, PartialEq)]
pub enum LexerError {
    InvalidCharInt(char),
    InvalidCharUint(char),
    InvalidCharBuffer(char),
    InvalidCharIdent(char),
    InvalidBufferLength(usize),
    UnknownEscapeChar(char),
    UnterminatedString,
    IllegalCharString(char),
    SingleSemiColon,
    UnknownSymbol(char),
}
