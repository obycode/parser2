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
        Self {
            input: input.chars(),
            next: '\0',
            offset: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn read_char(&mut self) {
        if self.next == '\n' {
            self.line = self.line + 1;
        }

        match self.input.next() {
            Some(ch) => self.next = ch,
            None => self.next = '\0',
        }
        self.offset = self.offset + 1;
        self.column = self.column + 1;
    }

    pub fn skip_whitespace(&mut self) -> bool {
        let mut found = false;
        while self.next != '\0' {
            match self.next {
                ' ' | '\t' | '\r' | '\n' => found = true,
                _ => break,
            }
            self.read_char();
        }
        found
    }

    pub fn read_line(&mut self) -> Vec<char> {
        let mut line = vec![];
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

    pub fn read_identifier(&mut self, first: char) -> Vec<char> {
        let mut ident = vec![first];

        loop {
            self.read_char();
            match self.next {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '!' | '?' => ident.push(self.next),
                _ => return ident,
            }
            self.read_char();
        }
    }

    pub fn read_unsigned(&mut self) -> u128 {
        let mut num: u128 = 0;
        while self.next.is_numeric() {
            let digit = self.next as u32 - '0' as u32;
            num = num * 10 + digit as u128;
            self.read_char();
        }
        num
    }

    pub fn read_integer(&mut self) -> i128 {
        let mut num: i128 = 0;
        while self.next.is_numeric() {
            let digit = self.next as u32 - '0' as u32;
            num = num * 10 + digit as i128;
            self.read_char();
        }
        num
    }

    pub fn read_token(&mut self) -> PlacedToken {
        self.skip_whitespace();
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
                self.read_char();
                if self.next.is_numeric() {
                    Token::Uint(self.read_unsigned())
                } else {
                    Token::Ident(self.read_identifier('u'))
                }
            }
            _ => {
                if self.next.is_ascii_alphabetic() {
                    Token::Ident(self.read_identifier(self.next))
                } else if self.next.is_numeric() {
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
                end_column: self.column as u32,
            },
            token,
        }
    }
}
