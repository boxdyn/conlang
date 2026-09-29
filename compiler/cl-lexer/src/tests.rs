use crate::Lexer;
use cl_token::*;

macro_rules! test_tkind {
    ($($f:ident {$($test:expr => $expect:expr),*$(,)?})*) => {$(
    #[test]
    fn $f() -> Result<(), $crate::LexError> {
        $(assert_eq!(
            {
                let mut out = vec![];
                let mut lexer = Lexer::new("".into(), $test);
                loop{match lexer.scan() {
                    Ok(token) => out.push(token.kind),
                    Err($crate::LexError { res: $crate::LexFailure::EOF, .. }) => break,
                    Err(e) => Err(e)?,
                }};
                out
            },
            dbg!($expect)
        ));*;
        Ok(())
    }
)*};
}

macro_rules! test_lexeme {
    ($($f:ident {$($test:expr => $expect:expr),*$(,)?})*) => {$(
    #[test]
    fn $f() -> Result<(), $crate::LexError> {
        $(assert_eq!(
            {
                let mut out = vec![];
                let mut lexer = Lexer::new("".into(), $test);
                loop{match lexer.scan() {
                    Ok(token) => out.push(token.lexeme),
                    Err($crate::LexError { res: $crate::LexFailure::EOF, .. }) => break,
                    Err(e) => Err(e)?,
                }};
                out
            },
            dbg!($expect)
        ));*;
        Ok(())
    }
)*};
}

mod ident {
    use super::*;
    macro_rules! ident {
        ($($id:literal),*) => {
            [$(Lexeme::String($id.into())),*]
        };
    }
    test_lexeme! {
        underscore { "_ _" => ident!["_", "_"] }
        unicode { "_ε ε_" => ident!["_ε", "ε_"] }
        many_underscore { "____________________________________" =>
        ident!["____________________________________"] }
    }
}
mod keyword {
    use super::*;
    macro_rules! kw {
        ($($k:ident),*) => {
            [ $(TKind::$k,)* ]
        };
    }
    test_tkind! {
        kw_break { "break break" => kw![Break, Break] }
        kw_continue { "continue continue" => kw![Continue, Continue] }
        kw_else { "else else" => kw![Else, Else] }
        kw_false { "false false" => kw![False, False] }
        kw_for { "for for" => kw![For, For] }
        kw_fn { "fn fn" => kw![Fn, Fn] }
        kw_if { "if if" => kw![If, If] }
        kw_in { "in in" => kw![In, In] }
        kw_let { "let let" => kw![Let, Let] }
        kw_return { "return return" => kw![Return, Return] }
        kw_true { "true true" => kw![True, True] }
        kw_while { "while while" => kw![While, While] }
        keywords { "break continue else false for fn if in let return true while" =>
            kw![Break, Continue, Else, False, For, Fn, If, In, Let, Return, True, While] }
    }
}
mod integer {
    use super::*;
    test_lexeme! {
        b36 {
            "0~0 0~1 0~l 0~6io 0~pa8" =>
            [Lexeme::Integer(0, 36), Lexeme::Integer(1, 36), Lexeme::Integer(21, 36), Lexeme::Integer(8448, 36), Lexeme::Integer(32768, 36)]
        }
        hex {
            "0x0 0x1 0x15 0x2100 0x8000" =>
            [Lexeme::Integer(0, 16), Lexeme::Integer(1, 16), Lexeme::Integer(21, 16), Lexeme::Integer(8448, 16), Lexeme::Integer(32768, 16)]
        }
        dec {
            "0d0 0d1 0d21 0d8448 0d32768" =>
            [Lexeme::Integer(0, 10), Lexeme::Integer(1, 10), Lexeme::Integer(21, 10), Lexeme::Integer(8448, 10), Lexeme::Integer(32768, 10)]
        }
        oct {
            "0o0 0o1 0o25 0o20400 0o100000" =>
            [Lexeme::Integer(0, 8), Lexeme::Integer(1, 8), Lexeme::Integer(21, 8), Lexeme::Integer(8448, 8), Lexeme::Integer(32768, 8)]
        }
        bin {
            "0b0 0b1 0b10101 0b10000100000000 0b1000000000000000" =>
            [Lexeme::Integer(0, 2), Lexeme::Integer(1, 2), Lexeme::Integer(21, 2), Lexeme::Integer(8448, 2), Lexeme::Integer(32768, 2)]
        }
        baseless {
            "0 1 21 8448 32768" =>
            [Lexeme::Integer(0, 10), Lexeme::Integer(1, 10), Lexeme::Integer(21, 10), Lexeme::Integer(8448, 10), Lexeme::Integer(32768, 10)]
        }
    }
}
mod string {
    use super::*;
    test_lexeme! {
        empty_string {
            "\"\"" =>
            [Lexeme::String("".into())]
        }
        unicode_string {
            "\"I 💙 🦈!\"" =>
            [Lexeme::String("I 💙 🦈!".into())]
        }
        escape_string {
            " \"This is a shark: \\u{1f988}\" " =>
            [Lexeme::String("This is a shark: 🦈".into())]
        }
    }
}
mod punct {
    macro_rules! op {
        ($op:ident) => {
            TKind::$op
        };
    }

    use super::*;
    test_tkind! {
        l_curly   { "{ {"   => [ op!(LCurly), op!(LCurly) ] }
        r_curly   { "} }"   => [ op!(RCurly), op!(RCurly) ] }
        l_brack   { "[ ["   => [ op!(LBrack), op!(LBrack) ] }
        r_brack   { "] ]"   => [ op!(RBrack), op!(RBrack) ] }
        l_paren   { "( ("   => [ op!(LParen), op!(LParen) ] }
        r_paren   { ") )"   => [ op!(RParen), op!(RParen) ] }
        amp       { "& &"   => [ op!(Amp), op!(Amp) ] }
        amp_amp   { "&& &&" => [ op!(AmpAmp), op!(AmpAmp) ] }
        amp_eq    { "&= &=" => [ op!(AmpEq), op!(AmpEq) ] }
        arrow     { "-> ->" => [ op!(Arrow), op!(Arrow)] }
        at        { "@ @"   => [ op!(At), op!(At)] }
        backslash { "\\ \\" => [ op!(Backslash), op!(Backslash)] }
        bang      { "! !"   => [ op!(Bang), op!(Bang)] }
        bangbang  { "!! !!" => [ op!(BangBang), op!(BangBang)] }
        bangeq    { "!= !=" => [ op!(BangEq), op!(BangEq)] }
        bar       { "| |"   => [ op!(Bar), op!(Bar)] }
        barbar    { "|| ||" => [ op!(BarBar), op!(BarBar)] }
        bareq     { "|= |=" => [ op!(BarEq), op!(BarEq)] }
        colon     { ": :"   => [ op!(Colon), op!(Colon)] }
        comma     { ", ,"   => [ op!(Comma), op!(Comma)] }
        dot       { ". ."   => [ op!(Dot), op!(Dot)] }
        dotdot    { ".. .." => [ op!(DotDot), op!(DotDot)] }
        dotdoteq  { "..= ..=" => [ op!(DotDotEq), op!(DotDotEq)] }
        eq        { "= ="   => [ op!(Eq), op!(Eq)] }
        eqeq      { "== ==" => [ op!(EqEq), op!(EqEq)] }
        fatarrow  { "=> =>" => [ op!(FatArrow), op!(FatArrow)] }
        grave     { "` `"   => [ op!(Grave), op!(Grave)] }
        gt        { "> >"   => [ op!(Gt), op!(Gt)] }
        gteq      { ">= >=" => [ op!(GtEq), op!(GtEq)] }
        gtgt      { ">> >>" => [ op!(GtGt), op!(GtGt)] }
        gtgteq    { ">>= >>=" => [ op!(GtGtEq), op!(GtGtEq)] }
        hash      { "# #"   => [ op!(Hash), op!(Hash)] }
        hashbang  { "#! #!" => [ op!(HashBang), op!(HashBang)] }
        lt        { "< <"   => [ op!(Lt), op!(Lt)] }
        lteq      { "<= <=" => [ op!(LtEq), op!(LtEq)] }
        ltlt      { "<< <<" => [ op!(LtLt), op!(LtLt)] }
        ltlteq    { "<<= <<=" => [ op!(LtLtEq), op!(LtLtEq)] }
        minus     { "- -"   => [ op!(Minus), op!(Minus)] }
        minuseq   { "-= -=" => [ op!(MinusEq), op!(MinusEq)] }
        plus      { "+ +"   => [ op!(Plus), op!(Plus)] }
        pluseq    { "+= +=" => [ op!(PlusEq), op!(PlusEq)] }
        question  { "? ?"   => [ op!(Question), op!(Question)] }
        rem       { "% %"   => [ op!(Rem), op!(Rem)] }
        remeq     { "%= %=" => [ op!(RemEq), op!(RemEq)] }
        semi      { "; ;"   => [ op!(Semi), op!(Semi)] }
        slash     { "/ /"   => [ op!(Slash), op!(Slash)] }
        slasheq   { "/= /=" => [ op!(SlashEq), op!(SlashEq)] }
        star      { "* *"   => [ op!(Star), op!(Star)] }
        stareq    { "*= *=" => [ op!(StarEq), op!(StarEq)] }
        tilde     { "~ ~"   => [ op!(Tilde), op!(Tilde)] }
        xor       { "^ ^"   => [ op!(Xor), op!(Xor)] }
        xoreq     { "^= ^=" => [ op!(XorEq), op!(XorEq)] }
        xorxor    { "^^ ^^" => [ op!(XorXor), op!(XorXor)] }
    }
}
