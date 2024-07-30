use crate::Lexer;
use cl_token::*;

macro test_lexer_output_type  ($($f:ident {$($test:expr => $expect:expr),*$(,)?})*) {$(
    #[test]
    fn $f() {$(
        assert_eq!(
            Lexer::new($test)
                .into_iter()
                .map(|t| t.unwrap().ty())
                .collect::<Vec<_>>(),
            dbg!($expect)
        );
    )*}
)*}

macro test_lexer_data_type  ($($f:ident {$($test:expr => $expect:expr),*$(,)?})*) {$(
    #[test]
    fn $f() {$(
        assert_eq!(
            Lexer::new($test)
                .into_iter()
                .map(|t| t.unwrap().into_data())
                .collect::<Vec<_>>(),
            dbg!($expect)
        );
    )*}
)*}

/// Convert an `[ expr, ... ]` into a `[ *, ... ]`
macro td ($($id:expr),*) {
    [$($id.into()),*]
}

mod ident {
    use super::*;
    macro ident ($($id:literal),*) {
        [$(TokenData::String($id.into())),*]
    }
    test_lexer_data_type! {
        underscore { "_ _" => ident!["_", "_"] }
        unicode { "_ε ε_" => ident!["_ε", "ε_"] }
        many_underscore { "____________________________________" =>
        ident!["____________________________________"] }
    }
}
mod keyword {
    use super::*;
    macro kw($($k:ident),*) {
        [ $(TokenKind::$k,)* ]
    }
    test_lexer_output_type! {
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
    test_lexer_data_type! {
        hex {
            "0x0 0x1 0x15 0x2100 0x8000" =>
            td![0x0, 0x1, 0x15, 0x2100, 0x8000]
        }
        dec {
            "0d0 0d1 0d21 0d8448 0d32768" =>
            td![0, 0x1, 0x15, 0x2100, 0x8000]
        }
        oct {
            "0o0 0o1 0o25 0o20400 0o100000" =>
            td![0x0, 0x1, 0x15, 0x2100, 0x8000]
        }
        bin {
            "0b0 0b1 0b10101 0b10000100000000 0b1000000000000000" =>
            td![0x0, 0x1, 0x15, 0x2100, 0x8000]
        }
        baseless {
            "0 1 21 8448 32768" =>
            td![0x0, 0x1, 0x15, 0x2100, 0x8000]
        }
    }
}
mod string {
    use super::*;
    test_lexer_data_type! {
        empty_string {
            "\"\"" =>
            td![String::from("")]
        }
        unicode_string {
            "\"I 💙 🦈!\"" =>
            td![String::from("I 💙 🦈!")]
        }
        escape_string {
            " \"This is a shark: \\u{1f988}\" " =>
            td![String::from("This is a shark: 🦈")]
        }
    }
}
mod punct {
    macro op($op:ident) {
        TokenKind::$op
    }

    use super::*;
    test_lexer_output_type! {
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
