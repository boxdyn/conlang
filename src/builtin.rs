use crate::inline_modules;
use cl_ast::{At, Expr};
use cl_interpret::{
    builtin::builtins, convalue::ConValue, env::Environment, error::Error, interpret::Interpret,
};
use cl_lexer::Lexer;
use cl_parser::Parser;
use std::{
    fs,
    io::{Write, stdout},
};

const PREAMBLE: &str = include_str!("preamble.cl");

pub fn get_env() -> Environment {
    let mut env = Environment::new();
    env.add_builtins(&builtins! {
        /// Lexes, parses, and evaluates an expression in the current env
        fn eval(string) @env {
            let string = match string.dereference_in(env)? {
                ConValue::Str(string) => string.to_ref(),
                ConValue::String(string) => string.as_str(),
                _ => Err(Error::TypeError("string", string.type_of()))?
            };

            match Parser::new(Lexer::new("eval".into(), string)).parse::<At<Expr>>(0).map(inline_modules) {
                Err(e) => Ok(ConValue::String(format!("{e}"))),
                Ok(v) => v.interpret(env),
            }
        }

        /// Puts a single character, flushing stdout
        fn putchar(ConValue::Char(c)) {
            let mut stdout = stdout().lock();
            let _ = write!(stdout, "{c}");
            let _ = stdout.flush();
            Ok(ConValue::Empty)
        }

        /// Gets a line of input from stdin
        fn get_line(prompt) @env {
            let prompt = match prompt.dereference_in(env)? {
                ConValue::Str(prompt) => prompt.to_ref(),
                ConValue::String(prompt) => prompt.as_str(),
                _ => Err(Error::TypeError("string", prompt.type_of()))?,
            };
            match repline::Repline::new("", prompt, "").read() {
                Ok(line) => Ok(ConValue::String(line)),
                Err(repline::Error::CtrlD(line)) => Ok(ConValue::String(line)),
                Err(repline::Error::CtrlC(_)) => Err(Error::Break(ConValue::Empty)),
                Err(e) => Ok(ConValue::String(e.to_string())),
            }
        }

        // TODO: low level file abstraction
        fn read_file(path) @env {
            let path = match path.dereference_in(env)? {
                ConValue::Str(path) => path.to_ref(),
                ConValue::String(path) => path.as_str(),
                _ => Err(Error::TypeError("string", path.type_of()))?,
            };
            fs::read_to_string(path).map_err(Error::BuiltinError)
        }

        fn write_file(path, data) @env {
            let path = match path.dereference_in(env)? {
                ConValue::Str(v) => v.to_ref(),
                ConValue::String(v) => v.as_str(),
                v => Err(Error::TypeError("string", v.type_of()))?,
            };
            let data = match data.dereference_in(env)? {
                ConValue::Str(v) => v.to_ref(),
                ConValue::String(v) => v.as_str(),
                v => Err(Error::TypeError("string", v.type_of()))?,
            };
            fs::write(path, data).map_err(Error::BuiltinError)
        }
    });

    if let Ok(code) = Parser::new(Lexer::new("".into(), PREAMBLE)).parse::<At<Expr>>(0) {
        code.interpret(&mut env).expect("PREAMBLE should not fail");
    }
    env
}
