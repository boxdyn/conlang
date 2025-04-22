//! The Result type, indicating a fallible operation.

pub enum Result<T, E> {
    Ok(T),
    Err(E),
}

impl Result {
    pub fn is_ok(&self) -> bool {
        match self {
            Ok(_) => true,
            Err(_) => false,
        }
    }
    pub fn is_err(&self) -> bool {
        match self {
            Ok(_) => false,
            Err(_) => true,
        }
    }
}
