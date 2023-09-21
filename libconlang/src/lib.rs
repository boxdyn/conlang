//! Conlang is an expression-based programming language

pub mod token {
    //! Stores a component of a file as a type and span
}

pub mod ast {
    //! Stores functions, data structure definitions, etc.
}

pub mod lexer {
    //! Converts a text file into tokens
}

pub mod parser {
    //! Parses tokens into an AST
}

pub mod interpreter {
    //! Interprets an AST as a program
}

#[cfg(test)]
mod tests {
    mod token {
        // TODO
    }
    mod ast {
        // TODO
    }
    mod lexer {
        // TODO
    }
    mod parser {
        // TODO
    }
    mod interpreter {
        // TODO
    }
}
