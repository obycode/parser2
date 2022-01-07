mod lexer;
use crate::lexer::token::Token;
use std::io::{self, BufRead, Read};

fn main() {
    let mut stdin = io::stdin();
    let mut input = String::new();
    if let Err(e) = stdin.read_to_string(&mut input) {
        println!("Error reading from stdin: {}", e);
        return;
    }
    let mut lex = lexer::Lexer::new(input.as_str());
    loop {
        let token = lex.read_token();
        println!(
            "{}:{}..{}:{}: {:?}",
            token.span.start_line,
            token.span.start_column,
            token.span.end_line,
            token.span.end_column,
            token.token
        );
        match token.token {
            Token::Eof => break,
            _ => (),
        }
    }
}
