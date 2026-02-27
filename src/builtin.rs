use crate::inline_modules;
use cl_ast::{At, Expr};
use cl_interpret::{builtin::builtins, convalue::ConValue, env::Environment, interpret::Interpret};
use cl_lexer::Lexer;
use cl_parser::Parser;

pub fn get_env() -> Environment {
    let mut env = Environment::new();
    env.add_builtins(&builtins! {
        /// Lexes, parses, and evaluates an expression in the current env
        fn eval(string) @env {
            use cl_interpret::error::Error;
            let string = match string {
                ConValue::Str(string) => string.to_ref(),
                ConValue::String(string) => string.as_str(),
                ConValue::Ref(v) => {
                    let string = v.get(env).cloned().unwrap_or_default();
                    return eval(env, &[string])
                }
                _ => Err(Error::TypeError("string", string.typename()))?
            };



            match Parser::new(Lexer::new("eval".into(), string)).parse::<At<Expr>>(0).map(inline_modules) {
                Err(e) => Ok(ConValue::String(format!("{e}"))),
                Ok(v) => v.interpret(env),
            }
        }

        fn putchar(ConValue::Char(c)) {
            print!("{c}");
            Ok(ConValue::Empty)
        }

        /// Gets a line of input from stdin
        fn get_line(prompt) {
            use cl_interpret::error::Error;
            let prompt = match prompt {
                ConValue::Str(prompt) => prompt.to_ref(),
                ConValue::String(prompt) => prompt.as_str(),
                _ => Err(Error::TypeError("string", prompt.typename()))?,
            };
            match repline::Repline::new("", prompt, "").read() {
                Ok(line) => Ok(ConValue::String(line)),
                Err(repline::Error::CtrlD(line)) => Ok(ConValue::String(line)),
                Err(repline::Error::CtrlC(_)) => Err(cl_interpret::error::Error::Break(ConValue::Empty)),
                Err(e) => Ok(ConValue::String(e.to_string())),
            }
        }
    });
    env
}
