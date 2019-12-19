use std::io::{self, BufRead, Write};

use lrlex::lrlex_mod;
use lrpar::lrpar_mod;

// Using `lrlex_mod!` brings the lexer for `calc.l` into scope. By default the module name will be
// `calc_l` (i.e. the file name, minus any extensions, with a suffix of `_l`).
lrlex_mod!("calc.l");
// Using `lrpar_mod!` brings the parser for `calc.y` into scope. By default the module name will be
// `calc_y` (i.e. the file name, minus any extensions, with a suffix of `_y`).
lrpar_mod!("calc.y");

use calc_y::Expr;

fn main() {
    // Get the `LexerDef` for the `calc` language.
    let lexerdef = calc_l::lexerdef();
    let stdin = io::stdin();
    loop {
        print!(">>> ");
        io::stdout().flush().ok();
        match stdin.lock().lines().next() {
            Some(Ok(ref l)) => {
                if l.trim().is_empty() {
                    continue;
                }
                // Now we create a lexer with the `lexer` method with which we can lex an input.
                let lexer = lexerdef.lexer(l);
                // Pass the lexer to the parser and lex and parse the input.
                let (res, errs) = calc_y::parse(&lexer);
                for e in errs {
                    println!("{}", e.pp(&lexer, &calc_y::token_epp));
                }
                if let Some(Ok(r)) = res {
                    if let Ok(i) = eval(r) {
                        println!("Result: {}", i);
                        continue;
                    }
                }
                eprintln!("Unable to evaluate expression.");
            }
            _ => break
        }
    }
}

fn eval(e: Expr) -> Result<u64, ()> {
    match e {
        Expr::Add { lhs, rhs } => Ok(eval(*lhs)? + eval(*rhs)?),
        Expr::Mul { lhs, rhs } => Ok(eval(*lhs)? * eval(*rhs)?),
        Expr::Number(s) => match s.parse::<u64>() {
            Ok(i) => Ok(i),
            Err(_) => {
                eprintln!("{} cannot be represented as a u64", s);
                Err(())
            }
        }
    }
}
