use crate::{inline_modules, pretty_error};
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

/// The Conlang interpreter preamble
///
///  ```rust,ignore
#[doc = include_str!("preamble.cl")]
/// ```
const PREAMBLE: &str = include_str!("preamble.cl");

pub fn get_env() -> Environment {
    let mut env = Environment::new();
    env.add_builtins(&builtins! {
        /// Lexes, parses, and evaluates an expression in the current env
        fn eval(string) @env {
            let string = match string.dereference_in(env)? {
                ConValue::Str(string) => string.to_ref(),
                ConValue::String(string) => string.as_str(),
                _ => Err(Error::TypeError("str", string.type_of(env)))?
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
            Ok(ConValue::Unit)
        }

        /// Gets a line of input from stdin
        fn get_line(prompt) @env {
            let prompt = match prompt.dereference_in(env)? {
                ConValue::Str(prompt) => prompt.to_ref(),
                ConValue::String(prompt) => prompt.as_str(),
                _ => Err(Error::TypeError("str", prompt.type_of(env)))?,
            };
            match repline::Repline::new("", prompt, "").read() {
                Ok(line) => Ok(ConValue::String(line)),
                Err(repline::Error::CtrlD(line)) => Ok(ConValue::String(line)),
                Err(repline::Error::CtrlC(_)) => Err(Error::Break(ConValue::Unit)),
                Err(e) => Ok(ConValue::String(e.to_string())),
            }
        }

        // TODO: low level file abstraction
        fn read_file(path) @env {
            let path = match path.dereference_in(env)? {
                ConValue::Str(path) => path.to_ref(),
                ConValue::String(path) => path.as_str(),
                _ => Err(Error::TypeError("str", path.type_of(env)))?,
            };
            fs::read_to_string(path).map_err(Error::BuiltinError)
        }

        fn write_file(path, data) @env {
            let path = match path.dereference_in(env)? {
                ConValue::Str(v) => v.to_ref(),
                ConValue::String(v) => v.as_str(),
                v => Err(Error::TypeError("str", v.type_of(env)))?,
            };
            let data = match data.dereference_in(env)? {
                ConValue::Str(v) => v.to_ref(),
                ConValue::String(v) => v.as_str(),
                v => Err(Error::TypeError("str", v.type_of(env)))?,
            };
            fs::write(path, data).map_err(Error::BuiltinError)
        }
    });
    match Parser::new(Lexer::new("preamble.cl".into(), PREAMBLE)).parse::<At<Expr>>(0) {
        Ok(code) => match code.interpret(&mut env) {
            Ok(_) => {}
            Err(Error { kind, span: Some(span) }) => pretty_error(span, PREAMBLE, kind),
            Err(e) => println!("{e}"),
        },
        Err(e) => pretty_error(e.span(), PREAMBLE, e),
    };
    env
}
