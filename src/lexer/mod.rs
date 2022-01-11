pub mod token;

use std::{char, str::Chars};
use token::{PlacedToken, Span, Token};

pub struct Lexer<'a> {
    input: Chars<'a>,
    next: char,
    offset: usize,
    pub line: usize,
    pub column: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut s = Self {
            input: input.chars(),
            next: 0 as char,
            offset: 0,
            line: 1,
            column: 0,
        };
        s.read_char();
        s
    }

    pub fn read_char(&mut self) {
        if self.next == '\n' {
            self.line = self.line + 1;
            self.column = 0;
        }

        match self.input.next() {
            Some(ch) => self.next = ch,
            None => self.next = '\0',
        }
        self.offset = self.offset + 1;
        self.column = self.column + 1;
    }

    pub fn skip_whitespace(&mut self) {
        while self.next != '\0' {
            match self.next {
                ' ' | '\t' | '\r' | '\n' => (),
                _ => break,
            }
            self.read_char();
        }
    }

    pub fn read_line(&mut self) -> String {
        let mut line = String::new();
        loop {
            match self.next {
                '\n' => {
                    self.read_char();
                    break;
                }
                '\0' => break,
                '\r' => (),
                ch => line.push(ch),
            }
            self.read_char();
        }
        line
    }

    pub fn read_identifier(&mut self, first: Option<char>) -> String {
        let mut ident = String::new();
        if let Some(first) = first {
            ident.push(first);
        }

        loop {
            match self.next {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '!' | '?' => ident.push(self.next),
                _ => return ident,
            }
            self.read_char();
        }
    }

    pub fn read_unsigned(&mut self) -> u128 {
        let mut num: u128 = 0;
        while self.next.is_ascii_digit() {
            let digit = self.next as u32 - '0' as u32;
            num = num * 10 + digit as u128;
            self.read_char();
        }
        num
    }

    pub fn read_integer(&mut self) -> i128 {
        let mut num: i128 = 0;
        while self.next.is_ascii_digit() {
            let digit = self.next as u32 - '0' as u32;
            num = num * 10 + digit as i128;
            self.read_char();
        }
        num
    }

    pub fn read_hex(&mut self) -> Option<Vec<u8>> {
        let mut bytes = vec![];
        loop {
            self.read_char();

            let f = self.next;
            if !f.is_ascii_hexdigit() {
                break;
            }

            self.read_char();
            let s = self.next;

            match (f.to_digit(16), s.to_digit(16)) {
                (None, _) => return None, //Err(HexError::BadCharacter(f)),
                (_, None) => return None, //Err(HexError::BadCharacter(s)),
                (Some(f), Some(s)) => {
                    bytes.push((f * 0x10 + s) as u8);
                }
            }
        }
        Some(bytes)
    }

    pub fn read_ascii_string(&mut self) -> Option<String> {
        let mut s = String::new();
        let mut escaped = false;
        self.read_char();
        loop {
            if escaped {
                let ch = match self.next {
                    '\\' => '\\',
                    '\"' => '\"',
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '0' => '\0',
                    _ => return None,
                };
                s.push(ch);
                escaped = false;
            } else {
                match self.next {
                    '"' => return Some(s),
                    '\\' => escaped = !escaped,
                    '\0' => return None,
                    _ => {
                        if !self.next.is_ascii() {
                            return None;
                        }
                        escaped = false;
                        s.push(self.next);
                    }
                }
            }
            self.read_char();
        }
    }

    pub fn read_utf8_string(&mut self) -> Option<String> {
        let mut s = String::new();
        let mut escaped = false;
        self.read_char();
        loop {
            if escaped {
                match self.next {
                    '\\' => s.push('\\'),
                    '\"' => s.push('\"'),
                    'n' => s.push('\n'),
                    't' => s.push('\t'),
                    'r' => s.push('\r'),
                    '0' => s.push('\0'),
                    'u' => s.push_str("\\u"),
                    _ => return None,
                };
                escaped = false;
            } else {
                match self.next {
                    '"' => return Some(s),
                    '\\' => escaped = !escaped,
                    '\0' => return None,
                    _ => {
                        escaped = false;
                        s.push(self.next);
                    }
                }
            }
            self.read_char();
        }
    }

    pub fn read_token(&mut self) -> PlacedToken {
        let start_line = self.line as u32;
        let start_column = self.column as u32;
        let mut advance = true;

        let token = match self.next {
            '\0' => Token::Eof,
            '(' => Token::Lparen,
            ')' => Token::Rparen,
            '{' => Token::Lbrace,
            '}' => Token::Rbrace,
            ':' => Token::Colon,
            '.' => Token::Dot,
            ',' => Token::Comma,
            '+' => Token::Plus,
            '-' => Token::Minus,
            '*' => Token::Multiply,
            '/' => Token::Divide,
            '<' => {
                self.read_char();
                if self.next == '=' {
                    Token::LessEqual
                } else {
                    advance = false;
                    Token::Less
                }
            }
            '>' => {
                self.read_char();
                if self.next == '=' {
                    Token::GreaterEqual
                } else {
                    advance = false;
                    Token::Greater
                }
            }
            ';' => {
                self.read_char();
                if self.next != ';' {
                    Token::Invalid
                } else {
                    self.read_char();
                    self.skip_whitespace();
                    let comment = self.read_line();
                    Token::Comment(comment)
                }
            }
            'u' => {
                advance = false;
                self.read_char();
                if self.next.is_ascii_digit() {
                    Token::Uint(self.read_unsigned())
                } else if self.next == '"' {
                    match self.read_utf8_string() {
                        Some(s) => Token::Utf8String(s),
                        None => Token::Invalid,
                    }
                } else {
                    Token::Ident(self.read_identifier(Some('u')))
                }
            }
            ' ' | '\t' | '\r' | '\n' => {
                self.skip_whitespace();
                advance = false;
                Token::Whitespace
            }
            '"' => {
                advance = false;
                match self.read_ascii_string() {
                    Some(s) => Token::AsciiString(s),
                    None => Token::Invalid,
                }
            }
            '0' => {
                advance = false;
                self.read_char();
                if self.next == 'x' {
                    match self.read_hex() {
                        Some(v) => Token::Bytes(v),
                        None => Token::Invalid,
                    }
                } else if self.next.is_ascii_digit() {
                    Token::Int(self.read_integer())
                } else {
                    Token::Int(0)
                }
            }
            _ => {
                advance = false;
                if self.next.is_ascii_alphabetic() {
                    Token::Ident(self.read_identifier(None))
                } else if self.next.is_ascii_digit() {
                    Token::Int(self.read_integer())
                } else {
                    Token::Invalid
                }
            }
        };

        if advance {
            self.read_char();
        }

        PlacedToken {
            span: Span {
                start_line,
                start_column,
                end_line: self.line as u32,
                end_column: (self.column - 1) as u32,
            },
            token,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_tokens() {
        let mut lexer = Lexer::new("");
        assert_eq!(lexer.read_token().token, Token::Eof);

        let mut lexer = Lexer::new(" ");
        assert_eq!(lexer.read_token().token, Token::Whitespace);

        let mut lexer = Lexer::new("\t");
        assert_eq!(lexer.read_token().token, Token::Whitespace);

        let mut lexer = Lexer::new("\n");
        assert_eq!(lexer.read_token().token, Token::Whitespace);

        let mut lexer = Lexer::new("\r");
        assert_eq!(lexer.read_token().token, Token::Whitespace);

        lexer = Lexer::new("(");
        assert_eq!(lexer.read_token().token, Token::Lparen);

        lexer = Lexer::new(")");
        assert_eq!(lexer.read_token().token, Token::Rparen);

        lexer = Lexer::new("{");
        assert_eq!(lexer.read_token().token, Token::Lbrace);

        lexer = Lexer::new("}");
        assert_eq!(lexer.read_token().token, Token::Rbrace);

        lexer = Lexer::new(":");
        assert_eq!(lexer.read_token().token, Token::Colon);

        lexer = Lexer::new(",");
        assert_eq!(lexer.read_token().token, Token::Comma);

        lexer = Lexer::new(".");
        assert_eq!(lexer.read_token().token, Token::Dot);

        lexer = Lexer::new("123");
        assert_eq!(lexer.read_token().token, Token::Int(123));

        lexer = Lexer::new("0123");
        assert_eq!(lexer.read_token().token, Token::Int(123));

        lexer = Lexer::new("0a");
        assert_eq!(lexer.read_token().token, Token::Int(0));
        assert_eq!(lexer.read_token().token, Token::Ident("a".to_string()));

        lexer = Lexer::new("1a");
        assert_eq!(lexer.read_token().token, Token::Int(1));

        lexer = Lexer::new("u123");
        assert_eq!(lexer.read_token().token, Token::Uint(123));

        lexer = Lexer::new("\"hello\"");
        assert_eq!(
            lexer.read_token().token,
            Token::AsciiString("hello".to_string())
        );

        lexer = Lexer::new("\"new\\nline\"");
        assert_eq!(
            lexer.read_token().token,
            Token::AsciiString("new\nline".to_string())
        );

        lexer = Lexer::new("\"quote \\\"this\\\"\"");
        assert_eq!(
            lexer.read_token().token,
            Token::AsciiString("quote \"this\"".to_string())
        );

        lexer = Lexer::new("\"\\r\\t\\0\\\\ ok\"");
        assert_eq!(
            lexer.read_token().token,
            Token::AsciiString("\r\t\0\\ ok".to_string())
        );

        lexer = Lexer::new("\"\\x\"");
        assert_eq!(lexer.read_token().token, Token::Invalid);

        lexer = Lexer::new("\"open");
        assert_eq!(lexer.read_token().token, Token::Invalid);

        lexer = Lexer::new("\"👎\"");
        assert_eq!(lexer.read_token().token, Token::Invalid);

        lexer = Lexer::new("\"\\u{1F600}\"");
        assert_eq!(lexer.read_token().token, Token::Invalid);

        lexer = Lexer::new("u\"\\u{1F600}\"");
        assert_eq!(
            lexer.read_token().token,
            Token::Utf8String("\\u{1F600}".to_string())
        );

        lexer = Lexer::new("u\"quote \\\"this\\\"\"");
        assert_eq!(
            lexer.read_token().token,
            Token::Utf8String("quote \"this\"".to_string())
        );

        lexer = Lexer::new("u\"\\n\\r\\t\\0\\\\ ok\"");
        assert_eq!(
            lexer.read_token().token,
            Token::Utf8String("\n\r\t\0\\ ok".to_string())
        );

        lexer = Lexer::new("u\"\\x\"");
        assert_eq!(lexer.read_token().token, Token::Invalid);

        lexer = Lexer::new("u\"open");
        assert_eq!(lexer.read_token().token, Token::Invalid);

        lexer = Lexer::new("0x123abc");
        if let Token::Bytes(v) = lexer.read_token().token {
            assert_eq!(v.len(), 3);
            assert_eq!(v[0], 0x12);
            assert_eq!(v[1], 0x3a);
            assert_eq!(v[2], 0xbc);
        } else {
            assert!(false);
        }

        lexer = Lexer::new("0xdefg");
        assert_eq!(lexer.read_token().token, Token::Invalid);

        lexer = Lexer::new("0xdef");
        assert_eq!(lexer.read_token().token, Token::Invalid);

        lexer = Lexer::new("0x00p5");
        if let Token::Bytes(v) = lexer.read_token().token {
            assert_eq!(v.len(), 1);
            assert_eq!(v[0], 0x0);
        } else {
            assert!(false);
        }
        assert_eq!(lexer.read_token().token, Token::Ident("p5".to_string()));

        lexer = Lexer::new("0xdef0 ");
        if let Token::Bytes(v) = lexer.read_token().token {
            assert_eq!(v.len(), 2);
            assert_eq!(v[0], 0xde);
            assert_eq!(v[1], 0xf0);
        } else {
            assert!(false);
        }

        lexer = Lexer::new("foo");
        assert_eq!(lexer.read_token().token, Token::Ident("foo".to_string()));

        lexer = Lexer::new("ubar");
        assert_eq!(lexer.read_token().token, Token::Ident("ubar".to_string()));

        lexer = Lexer::new("+");
        assert_eq!(lexer.read_token().token, Token::Plus);

        lexer = Lexer::new("-");
        assert_eq!(lexer.read_token().token, Token::Minus);

        lexer = Lexer::new("*");
        assert_eq!(lexer.read_token().token, Token::Multiply);

        lexer = Lexer::new("/");
        assert_eq!(lexer.read_token().token, Token::Divide);

        lexer = Lexer::new("<");
        assert_eq!(lexer.read_token().token, Token::Less);

        lexer = Lexer::new("<=");
        assert_eq!(lexer.read_token().token, Token::LessEqual);

        lexer = Lexer::new(">");
        assert_eq!(lexer.read_token().token, Token::Greater);

        lexer = Lexer::new(">=");
        assert_eq!(lexer.read_token().token, Token::GreaterEqual);

        lexer = Lexer::new(";; this is a comment");
        assert_eq!(
            lexer.read_token().token,
            Token::Comment("this is a comment".to_string())
        );

        lexer = Lexer::new(";; this is a comment\nthis is not");
        assert_eq!(
            lexer.read_token().token,
            Token::Comment("this is a comment".to_string())
        );

        lexer = Lexer::new(";; this is a comment\r\n");
        assert_eq!(
            lexer.read_token().token,
            Token::Comment("this is a comment".to_string())
        );

        lexer = Lexer::new("; this is not a comment");
        assert_eq!(lexer.read_token().token, Token::Invalid);

        lexer = Lexer::new("~");
        assert_eq!(lexer.read_token().token, Token::Invalid);
    }

    #[test]
    fn read_multiple_tokens() {
        let mut lexer = Lexer::new(" +321");
        assert_eq!(lexer.read_token().token, Token::Whitespace);
        assert_eq!(lexer.read_token().token, Token::Plus);
        assert_eq!(lexer.read_token().token, Token::Int(321));
        assert_eq!(lexer.read_token().token, Token::Eof);
        assert_eq!(lexer.read_token().token, Token::Eof);
    }
}
