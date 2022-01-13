pub mod error;
pub mod token;

use std::{char, str::Chars};
use token::{PlacedToken, Span, Token};

use crate::lexer::error::LexerError;

pub struct Lexer<'a> {
    input: Chars<'a>,
    next: char,
    offset: usize,
    pub line: usize,
    pub column: usize,
    pub last_line: usize,
    pub last_column: usize,
    // While lexing, collect errors and continue on (when possible)
    errors: Vec<PlacedToken>,
}

fn is_separator(ch: char) -> bool {
    match ch {
        '(' | ')' | '{' | '}' | ',' | ':' | '\0' => true,
        _ => ch.is_ascii_whitespace(),
    }
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut s = Self {
            input: input.chars(),
            next: 0 as char,
            offset: 0,
            line: 1,
            column: 0,
            last_line: 0,
            last_column: 0,
            errors: vec![],
        };
        s.read_char(); // Initialize with the first character
        s
    }

    pub fn read_char(&mut self) {
        self.last_line = self.line;
        self.last_column = self.column;

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

    fn proceed_through_error(&mut self, err: LexerError) {
        let start_line = self.line as u32;
        let start_column = self.column as u32;
        while !is_separator(self.next) {
            self.read_char();
        }
        self.errors.push(PlacedToken {
            span: Span {
                start_line,
                start_column,
                end_line: self.last_line as u32,
                end_column: self.last_column as u32,
            },
            token: Token::Error(err),
        });
    }

    pub fn read_identifier(&mut self, first: Option<char>) -> String {
        let mut ident = String::new();
        if let Some(first) = first {
            ident.push(first);
        }

        loop {
            match self.next {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '!' | '?' => ident.push(self.next),
                _ => {
                    if is_separator(self.next) {
                        return ident;
                    } else {
                        self.proceed_through_error(LexerError::InvalidCharIdent(self.next));
                        return ident;
                    }
                }
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
        if !is_separator(self.next) {
            self.proceed_through_error(LexerError::InvalidCharUint(self.next));
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
        if !is_separator(self.next) {
            self.proceed_through_error(LexerError::InvalidCharInt(self.next));
        }
        num
    }

    pub fn read_hex(&mut self) -> Vec<u8> {
        let start_line = self.line as u32;
        let start_column = (self.column - 1) as u32;
        let mut bytes = vec![];
        loop {
            self.read_char();

            let f = self.next;
            if !f.is_ascii_hexdigit() {
                if !is_separator(f) {
                    self.proceed_through_error(LexerError::InvalidCharBuffer(f));
                }
                return bytes;
            }

            self.read_char();
            let s = self.next;
            if !s.is_ascii_hexdigit() {
                if is_separator(s) {
                    self.errors.push(PlacedToken {
                        span: Span {
                            start_line,
                            start_column,
                            end_line: self.last_line as u32,
                            end_column: self.last_column as u32,
                        },
                        token: Token::Error(LexerError::InvalidBufferLength(bytes.len() * 2 + 1)),
                    });
                } else {
                    self.proceed_through_error(LexerError::InvalidCharBuffer(s));
                }
                return bytes;
            }

            bytes.push((f.to_digit(16).unwrap() * 0x10 + s.to_digit(16).unwrap()) as u8);
        }
    }

    pub fn read_ascii_string(&mut self) -> String {
        let start_line = self.line as u32;
        let start_column = self.column as u32;
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
                    _ => {
                        self.errors.push(PlacedToken {
                            span: Span {
                                start_line: self.last_line as u32,
                                start_column: self.last_column as u32,
                                end_line: self.line as u32,
                                end_column: self.column as u32,
                            },
                            token: Token::Error(LexerError::UnknownEscapeChar(self.next)),
                        });
                        '�'
                    }
                };
                s.push(ch);
                escaped = false;
            } else {
                match self.next {
                    '"' => {
                        self.read_char();
                        return s;
                    }
                    '\\' => escaped = !escaped,
                    '\0' => {
                        self.errors.push(PlacedToken {
                            span: Span {
                                start_line: start_line,
                                start_column: start_column,
                                end_line: self.last_line as u32,
                                end_column: self.last_column as u32,
                            },
                            token: Token::Error(LexerError::UnterminatedString),
                        });
                        return s;
                    }
                    _ => {
                        if !self.next.is_ascii() {
                            self.errors.push(PlacedToken {
                                span: Span {
                                    start_line: self.line as u32,
                                    start_column: self.column as u32,
                                    end_line: self.line as u32,
                                    end_column: self.column as u32,
                                },
                                token: Token::Error(LexerError::IllegalCharString(self.next)),
                            });
                        }
                        escaped = false;
                        s.push(self.next);
                    }
                }
            }
            self.read_char();
        }
    }

    pub fn read_utf8_string(&mut self) -> String {
        let start_line = self.last_line as u32;
        let start_column = self.last_column as u32;
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
                    _ => {
                        self.errors.push(PlacedToken {
                            span: Span {
                                start_line: self.last_line as u32,
                                start_column: self.last_column as u32,
                                end_line: self.line as u32,
                                end_column: self.column as u32,
                            },
                            token: Token::Error(LexerError::UnknownEscapeChar(self.next)),
                        });
                        s.push('�');
                    }
                };
                escaped = false;
            } else {
                match self.next {
                    '"' => {
                        self.read_char();
                        return s;
                    }
                    '\\' => escaped = !escaped,
                    '\0' => {
                        self.errors.push(PlacedToken {
                            span: Span {
                                start_line,
                                start_column,
                                end_line: self.line as u32,
                                end_column: self.column as u32,
                            },
                            token: Token::Error(LexerError::UnterminatedString),
                        });
                        return s;
                    }
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
                    // If there is just one ';', report an error but continue
                    //  parsing as if there were two (a comment).
                    self.errors.push(PlacedToken {
                        span: Span {
                            start_line: self.last_line as u32,
                            start_column: self.last_column as u32,
                            end_line: self.last_line as u32,
                            end_column: self.last_column as u32,
                        },
                        token: Token::Error(LexerError::SingleSemiColon),
                    });
                } else {
                    self.read_char();
                }
                advance = false;
                self.skip_whitespace();
                let comment = self.read_line();
                Token::Comment(comment)
            }
            'u' => {
                advance = false;
                self.read_char();
                if self.next.is_ascii_digit() {
                    Token::Uint(self.read_unsigned())
                } else if self.next == '"' {
                    Token::Utf8String(self.read_utf8_string())
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
                Token::AsciiString(self.read_ascii_string())
            }
            '0' => {
                advance = false;
                self.read_char();
                if self.next == 'x' {
                    Token::Bytes(self.read_hex())
                } else if self.next.is_ascii_digit() {
                    Token::Int(self.read_integer())
                } else if is_separator(self.next) {
                    Token::Int(0)
                } else {
                    self.proceed_through_error(LexerError::InvalidCharInt(self.next));
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
                    self.errors.push(PlacedToken {
                        span: Span {
                            start_line: self.line as u32,
                            start_column: self.column as u32,
                            end_line: self.line as u32,
                            end_column: self.column as u32,
                        },
                        token: Token::Error(LexerError::UnknownSymbol(self.next)),
                    });
                    Token::Placeholder
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
                end_line: self.last_line as u32,
                end_column: self.last_column as u32,
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

        lexer = Lexer::new("0");
        assert_eq!(lexer.read_token().token, Token::Int(0));

        lexer = Lexer::new("0a");
        assert_eq!(lexer.read_token().token, Token::Int(0));
        assert_eq!(lexer.errors.len(), 1);
        assert_eq!(
            lexer.errors[0].token,
            Token::Error(LexerError::InvalidCharInt('a'))
        );

        lexer = Lexer::new("56789*");
        assert_eq!(lexer.read_token().token, Token::Int(56789));
        assert_eq!(lexer.errors.len(), 1);
        assert_eq!(
            lexer.errors[0].token,
            Token::Error(LexerError::InvalidCharInt('*'))
        );

        lexer = Lexer::new("u123");
        assert_eq!(lexer.read_token().token, Token::Uint(123));

        lexer = Lexer::new("u1a");
        assert_eq!(lexer.read_token().token, Token::Uint(1));
        assert_eq!(lexer.errors.len(), 1);
        assert_eq!(
            lexer.errors[0].token,
            Token::Error(LexerError::InvalidCharUint('a'))
        );

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
        assert_eq!(
            lexer.read_token().token,
            Token::AsciiString("�".to_string())
        );
        assert_eq!(lexer.errors.len(), 1);
        assert_eq!(
            lexer.errors[0].token,
            Token::Error(LexerError::UnknownEscapeChar('x'))
        );

        lexer = Lexer::new("\"open");
        assert_eq!(
            lexer.read_token().token,
            Token::AsciiString("open".to_string())
        );
        assert_eq!(lexer.errors.len(), 1);
        assert_eq!(
            lexer.errors[0].token,
            Token::Error(LexerError::UnterminatedString)
        );

        lexer = Lexer::new("\"👎\"");
        assert_eq!(
            lexer.read_token().token,
            Token::AsciiString("👎".to_string())
        );
        assert_eq!(lexer.errors.len(), 1);
        assert_eq!(
            lexer.errors[0].token,
            Token::Error(LexerError::IllegalCharString('👎'))
        );

        lexer = Lexer::new("\"\\u{1F600}\"");
        assert_eq!(
            lexer.read_token().token,
            Token::AsciiString("�{1F600}".to_string())
        );
        assert_eq!(lexer.errors.len(), 1);
        assert_eq!(
            lexer.errors[0].token,
            Token::Error(LexerError::UnknownEscapeChar('u'))
        );

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
        assert_eq!(lexer.read_token().token, Token::Utf8String("�".to_string()));
        assert_eq!(lexer.errors.len(), 1);
        assert_eq!(
            lexer.errors[0].token,
            Token::Error(LexerError::UnknownEscapeChar('x'))
        );

        lexer = Lexer::new("u\"open");
        assert_eq!(
            lexer.read_token().token,
            Token::Utf8String("open".to_string())
        );
        assert_eq!(lexer.errors.len(), 1);
        assert_eq!(
            lexer.errors[0].token,
            Token::Error(LexerError::UnterminatedString)
        );

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
        if let Token::Bytes(v) = lexer.read_token().token {
            assert_eq!(v.len(), 1);
            assert_eq!(v[0], 0xde);
        } else {
            assert!(false);
        }
        assert_eq!(lexer.errors.len(), 1);
        assert_eq!(
            lexer.errors[0].token,
            Token::Error(LexerError::InvalidCharBuffer('g'))
        );

        lexer = Lexer::new("0xdef");
        if let Token::Bytes(v) = lexer.read_token().token {
            assert_eq!(v.len(), 1);
            assert_eq!(v[0], 0xde);
        } else {
            assert!(false);
        }
        assert_eq!(lexer.errors.len(), 1);
        assert_eq!(
            lexer.errors[0].token,
            Token::Error(LexerError::InvalidBufferLength(3))
        );

        lexer = Lexer::new("0x00p5");
        if let Token::Bytes(v) = lexer.read_token().token {
            assert_eq!(v.len(), 1);
            assert_eq!(v[0], 0x0);
        } else {
            assert!(false);
        }
        assert_eq!(lexer.errors.len(), 1);
        assert_eq!(
            lexer.errors[0].token,
            Token::Error(LexerError::InvalidCharBuffer('p'))
        );

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

        lexer = Lexer::new("baz👍buz");
        assert_eq!(lexer.read_token().token, Token::Ident("baz".to_string()));
        assert_eq!(lexer.errors.len(), 1);
        assert_eq!(
            lexer.errors[0].token,
            Token::Error(LexerError::InvalidCharIdent('👍'))
        );

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
        assert_eq!(
            lexer.read_token().token,
            Token::Comment("this is not a comment".to_string())
        );
        assert_eq!(lexer.errors.len(), 1);
        assert_eq!(
            lexer.errors[0].token,
            Token::Error(LexerError::SingleSemiColon)
        );

        lexer = Lexer::new("~");
        assert_eq!(lexer.read_token().token, Token::Placeholder);
        assert_eq!(lexer.errors.len(), 1);
        assert_eq!(
            lexer.errors[0].token,
            Token::Error(LexerError::UnknownSymbol('~'))
        );
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

    #[test]
    fn check_span() {
        let mut lexer = Lexer::new(
            r#"
 (foo)
    }1234{abc
        +-*/    < <=       >
>=.: ;; comment
   "hello" u"world"     0x0123456789abcdeffedcba9876543210
	

   foo-bar_
"#,
        );
        let mut token = lexer.read_token();
        assert_eq!(token.token, Token::Whitespace);
        assert_eq!(
            token.span,
            Span {
                start_line: 1,
                start_column: 1,
                end_line: 2,
                end_column: 1
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Lparen);
        assert_eq!(
            token.span,
            Span {
                start_line: 2,
                start_column: 2,
                end_line: 2,
                end_column: 2
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Ident("foo".to_string()));
        assert_eq!(
            token.span,
            Span {
                start_line: 2,
                start_column: 3,
                end_line: 2,
                end_column: 5
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Rparen);
        assert_eq!(
            token.span,
            Span {
                start_line: 2,
                start_column: 6,
                end_line: 2,
                end_column: 6
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Whitespace);
        assert_eq!(
            token.span,
            Span {
                start_line: 2,
                start_column: 7,
                end_line: 3,
                end_column: 4
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Rbrace);
        assert_eq!(
            token.span,
            Span {
                start_line: 3,
                start_column: 5,
                end_line: 3,
                end_column: 5
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Int(1234));
        assert_eq!(
            token.span,
            Span {
                start_line: 3,
                start_column: 6,
                end_line: 3,
                end_column: 9
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Lbrace);
        assert_eq!(
            token.span,
            Span {
                start_line: 3,
                start_column: 10,
                end_line: 3,
                end_column: 10
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Ident("abc".to_string()));
        assert_eq!(
            token.span,
            Span {
                start_line: 3,
                start_column: 11,
                end_line: 3,
                end_column: 13
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Whitespace);
        assert_eq!(
            token.span,
            Span {
                start_line: 3,
                start_column: 14,
                end_line: 4,
                end_column: 8
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Plus);
        assert_eq!(
            token.span,
            Span {
                start_line: 4,
                start_column: 9,
                end_line: 4,
                end_column: 9
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Minus);
        assert_eq!(
            token.span,
            Span {
                start_line: 4,
                start_column: 10,
                end_line: 4,
                end_column: 10
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Multiply);
        assert_eq!(
            token.span,
            Span {
                start_line: 4,
                start_column: 11,
                end_line: 4,
                end_column: 11
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Divide);
        assert_eq!(
            token.span,
            Span {
                start_line: 4,
                start_column: 12,
                end_line: 4,
                end_column: 12
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Whitespace);
        assert_eq!(
            token.span,
            Span {
                start_line: 4,
                start_column: 13,
                end_line: 4,
                end_column: 16
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Less);
        assert_eq!(
            token.span,
            Span {
                start_line: 4,
                start_column: 17,
                end_line: 4,
                end_column: 17
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Whitespace);
        assert_eq!(
            token.span,
            Span {
                start_line: 4,
                start_column: 18,
                end_line: 4,
                end_column: 18
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::LessEqual);
        assert_eq!(
            token.span,
            Span {
                start_line: 4,
                start_column: 19,
                end_line: 4,
                end_column: 20
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Whitespace);
        assert_eq!(
            token.span,
            Span {
                start_line: 4,
                start_column: 21,
                end_line: 4,
                end_column: 27
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Greater);
        assert_eq!(
            token.span,
            Span {
                start_line: 4,
                start_column: 28,
                end_line: 4,
                end_column: 28
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Whitespace);
        assert_eq!(
            token.span,
            Span {
                start_line: 4,
                start_column: 29,
                end_line: 4,
                end_column: 29
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::GreaterEqual);
        assert_eq!(
            token.span,
            Span {
                start_line: 5,
                start_column: 1,
                end_line: 5,
                end_column: 2
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Dot);
        assert_eq!(
            token.span,
            Span {
                start_line: 5,
                start_column: 3,
                end_line: 5,
                end_column: 3
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Colon);
        assert_eq!(
            token.span,
            Span {
                start_line: 5,
                start_column: 4,
                end_line: 5,
                end_column: 4
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Whitespace);
        assert_eq!(
            token.span,
            Span {
                start_line: 5,
                start_column: 5,
                end_line: 5,
                end_column: 5
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Comment("comment".to_string()));
        assert_eq!(
            token.span,
            Span {
                start_line: 5,
                start_column: 6,
                end_line: 5,
                end_column: 15
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Whitespace);
        assert_eq!(
            token.span,
            Span {
                start_line: 5,
                start_column: 16,
                end_line: 6,
                end_column: 3
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::AsciiString("hello".to_string()));
        assert_eq!(
            token.span,
            Span {
                start_line: 6,
                start_column: 4,
                end_line: 6,
                end_column: 10
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Whitespace);
        assert_eq!(
            token.span,
            Span {
                start_line: 6,
                start_column: 11,
                end_line: 6,
                end_column: 11
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Utf8String("world".to_string()));
        assert_eq!(
            token.span,
            Span {
                start_line: 6,
                start_column: 12,
                end_line: 6,
                end_column: 19
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Whitespace);
        assert_eq!(
            token.span,
            Span {
                start_line: 6,
                start_column: 20,
                end_line: 6,
                end_column: 24
            }
        );

        token = lexer.read_token();
        if let Token::Bytes(v) = token.token {
            assert_eq!(v.len(), 16);
        } else {
            assert!(false);
        }
        assert_eq!(
            token.span,
            Span {
                start_line: 6,
                start_column: 25,
                end_line: 6,
                end_column: 58
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Whitespace);
        assert_eq!(
            token.span,
            Span {
                start_line: 6,
                start_column: 59,
                end_line: 9,
                end_column: 3
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Ident("foo-bar_".to_string()));
        assert_eq!(
            token.span,
            Span {
                start_line: 9,
                start_column: 4,
                end_line: 9,
                end_column: 11
            }
        );

        token = lexer.read_token();
        assert_eq!(token.token, Token::Whitespace);
        assert_eq!(
            token.span,
            Span {
                start_line: 9,
                start_column: 12,
                end_line: 9,
                end_column: 12
            }
        );
    }

    #[test]
    fn check_error_span() {
        let mut lexer = Lexer::new("0a 123");
        lexer.read_token();
        assert_eq!(
            lexer.errors[0].span,
            Span {
                start_line: 1,
                start_column: 2,
                end_line: 1,
                end_column: 2
            }
        );

        lexer = Lexer::new("56789* foo");
        lexer.read_token();
        assert_eq!(
            lexer.errors[0].span,
            Span {
                start_line: 1,
                start_column: 6,
                end_line: 1,
                end_column: 6
            }
        );

        lexer = Lexer::new("u1a *");
        lexer.read_token();
        assert_eq!(
            lexer.errors[0].span,
            Span {
                start_line: 1,
                start_column: 3,
                end_line: 1,
                end_column: 3
            }
        );

        lexer = Lexer::new("\"\\x\"(");
        lexer.read_token();
        assert_eq!(
            lexer.errors[0].span,
            Span {
                start_line: 1,
                start_column: 2,
                end_line: 1,
                end_column: 3
            }
        );

        lexer = Lexer::new("\"open");
        lexer.read_token();
        assert_eq!(
            lexer.errors[0].span,
            Span {
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 5
            }
        );

        lexer = Lexer::new("\" this is 👎!\"");
        lexer.read_token();
        assert_eq!(
            lexer.errors[0].span,
            Span {
                start_line: 1,
                start_column: 11,
                end_line: 1,
                end_column: 11
            }
        );

        lexer = Lexer::new("\"\\u{1F600}\"");
        lexer.read_token();
        assert_eq!(
            lexer.errors[0].span,
            Span {
                start_line: 1,
                start_column: 2,
                end_line: 1,
                end_column: 3
            }
        );

        lexer = Lexer::new("u\"\\x ok\"");
        lexer.read_token();
        assert_eq!(
            lexer.errors[0].span,
            Span {
                start_line: 1,
                start_column: 3,
                end_line: 1,
                end_column: 4
            }
        );

        lexer = Lexer::new("u\"open");
        lexer.read_token();
        assert_eq!(
            lexer.errors[0].span,
            Span {
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 7
            }
        );

        lexer = Lexer::new("0xdefg");
        lexer.read_token();
        assert_eq!(
            lexer.errors[0].span,
            Span {
                start_line: 1,
                start_column: 6,
                end_line: 1,
                end_column: 6
            }
        );

        lexer = Lexer::new("0xdef");
        lexer.read_token();
        assert_eq!(
            lexer.errors[0].span,
            Span {
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 5
            }
        );

        lexer = Lexer::new("0x00p5");
        lexer.read_token();
        assert_eq!(
            lexer.errors[0].span,
            Span {
                start_line: 1,
                start_column: 5,
                end_line: 1,
                end_column: 6
            }
        );

        lexer = Lexer::new("baz👍buz");
        lexer.read_token();
        assert_eq!(
            lexer.errors[0].span,
            Span {
                start_line: 1,
                start_column: 4,
                end_line: 1,
                end_column: 7
            }
        );

        lexer = Lexer::new("; this is not a comment");
        lexer.read_token();
        assert_eq!(
            lexer.errors[0].span,
            Span {
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 1
            }
        );

        lexer = Lexer::new("123 ~ abc");
        lexer.read_token();
        lexer.read_token();
        lexer.read_token();
        assert_eq!(
            lexer.errors[0].span,
            Span {
                start_line: 1,
                start_column: 5,
                end_line: 1,
                end_column: 5
            }
        );
    }
}
