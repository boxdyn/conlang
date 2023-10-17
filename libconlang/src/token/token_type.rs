//! Trait impls and helper functions for [Type] and [Keyword]
use super::{Keyword, Type};
use std::fmt::Display;

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Invalid => Display::fmt("invalid", f),
            Type::Comment => Display::fmt("comment", f),
            Type::Identifier => Display::fmt("identifier", f),
            Type::Keyword(k) => Display::fmt(k, f),
            Type::Integer => Display::fmt("integer literal", f),
            Type::Float => Display::fmt("float literal", f),
            Type::String => Display::fmt("string literal", f),
            Type::Character => Display::fmt("char literal", f),
            Type::LCurly => Display::fmt("left curly", f),
            Type::RCurly => Display::fmt("right curly", f),
            Type::LBrack => Display::fmt("left brack", f),
            Type::RBrack => Display::fmt("right brack", f),
            Type::LParen => Display::fmt("left paren", f),
            Type::RParen => Display::fmt("right paren", f),
            Type::Lsh => Display::fmt("shift left", f),
            Type::Rsh => Display::fmt("shift right", f),
            Type::AmpAmp => Display::fmt("and-and", f),
            Type::BarBar => Display::fmt("or-or", f),
            Type::NotNot => Display::fmt("not-not", f),
            Type::CatEar => Display::fmt("cat-ears", f),
            Type::EqEq => Display::fmt("equal to", f),
            Type::GtEq => Display::fmt("greater than or equal to", f),
            Type::LtEq => Display::fmt("less than or equal to", f),
            Type::NotEq => Display::fmt("not equal to", f),
            Type::StarEq => Display::fmt("star-assign", f),
            Type::DivEq => Display::fmt("div-assign", f),
            Type::RemEq => Display::fmt("rem-assign", f),
            Type::AddEq => Display::fmt("add-assign", f),
            Type::SubEq => Display::fmt("sub-assign", f),
            Type::AndEq => Display::fmt("and-assign", f),
            Type::OrEq => Display::fmt("or-assign", f),
            Type::XorEq => Display::fmt("xor-assign", f),
            Type::LshEq => Display::fmt("shift left-assign", f),
            Type::RshEq => Display::fmt("shift right-assign", f),
            Type::Arrow => Display::fmt("arrow", f),
            Type::FatArrow => Display::fmt("fat arrow", f),
            Type::Semi => Display::fmt("ignore", f),
            Type::Dot => Display::fmt("dot", f),
            Type::Star => Display::fmt("star", f),
            Type::Div => Display::fmt("div", f),
            Type::Plus => Display::fmt("add", f),
            Type::Minus => Display::fmt("sub", f),
            Type::Rem => Display::fmt("rem", f),
            Type::Bang => Display::fmt("bang", f),
            Type::Eq => Display::fmt("assign", f),
            Type::Lt => Display::fmt("less than", f),
            Type::Gt => Display::fmt("greater than", f),
            Type::Amp => Display::fmt("and", f),
            Type::Bar => Display::fmt("or", f),
            Type::Xor => Display::fmt("xor", f),
            Type::Hash => Display::fmt("hash", f),
            Type::At => Display::fmt("at", f),
            Type::Colon => Display::fmt("colon", f),
            Type::Backslash => Display::fmt("backslash", f),
            Type::Question => Display::fmt("huh?", f),
            Type::Comma => Display::fmt("comma", f),
            Type::Tilde => Display::fmt("tilde", f),
            Type::Grave => Display::fmt("grave", f),
        }
    }
}

impl Display for Keyword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Break => Display::fmt("break", f),
            Self::Continue => Display::fmt("continue", f),
            Self::Else => Display::fmt("else", f),
            Self::False => Display::fmt("false", f),
            Self::For => Display::fmt("for", f),
            Self::Fn => Display::fmt("fn", f),
            Self::If => Display::fmt("if", f),
            Self::In => Display::fmt("in", f),
            Self::Let => Display::fmt("let", f),
            Self::Return => Display::fmt("return", f),
            Self::True => Display::fmt("true", f),
            Self::While => Display::fmt("while", f),
        }
    }
}
