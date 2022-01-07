mod lexer;
use crate::lexer::token::Token;

fn main() {
    let mut lex = lexer::Lexer::new("(define-data-var current uint u0)");
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
