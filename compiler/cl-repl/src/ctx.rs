use cl_interpret::{
    env::Environment, error::IResult, interpret::Interpret, convalue::ConValue,
};

#[derive(Clone, Debug)]
pub struct Context {
    pub env: Environment,
}

impl Context {
    pub fn new() -> Self {
        Self { env: Environment::new() }
    }
    pub fn with_env(env: Environment) -> Self {
        Self { env }
    }
    pub fn run(&mut self, code: &impl Interpret) -> IResult<ConValue> {
        code.interpret(&mut self.env)
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
