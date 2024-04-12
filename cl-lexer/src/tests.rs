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
        [$(TokenData::Identifier($id.into())),*]
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
    use super::*;
    test_lexer_output_type! {
        l_curly   { "{ {"   => [ TokenKind::LCurly, TokenKind::LCurly ] }
        r_curly   { "} }"   => [ TokenKind::RCurly, TokenKind::RCurly ] }
        l_brack   { "[ ["   => [ TokenKind::LBrack, TokenKind::LBrack ] }
        r_brack   { "] ]"   => [ TokenKind::RBrack, TokenKind::RBrack ] }
        l_paren   { "( ("   => [ TokenKind::LParen, TokenKind::LParen ] }
        r_paren   { ") )"   => [ TokenKind::RParen, TokenKind::RParen ] }
        amp       { "& &"   => [ TokenKind::Amp, TokenKind::Amp ] }
        amp_amp   { "&& &&" => [ TokenKind::AmpAmp, TokenKind::AmpAmp ] }
        amp_eq    { "&= &=" => [ TokenKind::AmpEq, TokenKind::AmpEq ] }
        arrow     { "-> ->" => [ TokenKind::Arrow, TokenKind::Arrow] }
        at        { "@ @"   => [ TokenKind::At, TokenKind::At] }
        backslash { "\\ \\" => [ TokenKind::Backslash, TokenKind::Backslash] }
        bang      { "! !"   => [ TokenKind::Bang, TokenKind::Bang] }
        bangbang  { "!! !!" => [ TokenKind::BangBang, TokenKind::BangBang] }
        bangeq    { "!= !=" => [ TokenKind::BangEq, TokenKind::BangEq] }
        bar       { "| |"   => [ TokenKind::Bar, TokenKind::Bar] }
        barbar    { "|| ||" => [ TokenKind::BarBar, TokenKind::BarBar] }
        bareq     { "|= |=" => [ TokenKind::BarEq, TokenKind::BarEq] }
        colon     { ": :"   => [ TokenKind::Colon, TokenKind::Colon] }
        comma     { ", ,"   => [ TokenKind::Comma, TokenKind::Comma] }
        dot       { ". ."   => [ TokenKind::Dot, TokenKind::Dot] }
        dotdot    { ".. .." => [ TokenKind::DotDot, TokenKind::DotDot] }
        dotdoteq  { "..= ..=" => [ TokenKind::DotDotEq, TokenKind::DotDotEq] }
        eq        { "= ="   => [ TokenKind::Eq, TokenKind::Eq] }
        eqeq      { "== ==" => [ TokenKind::EqEq, TokenKind::EqEq] }
        fatarrow  { "=> =>" => [ TokenKind::FatArrow, TokenKind::FatArrow] }
        grave     { "` `"   => [ TokenKind::Grave, TokenKind::Grave] }
        gt        { "> >"   => [ TokenKind::Gt, TokenKind::Gt] }
        gteq      { ">= >=" => [ TokenKind::GtEq, TokenKind::GtEq] }
        gtgt      { ">> >>" => [ TokenKind::GtGt, TokenKind::GtGt] }
        gtgteq    { ">>= >>=" => [ TokenKind::GtGtEq, TokenKind::GtGtEq] }
        hash      { "# #"   => [ TokenKind::Hash, TokenKind::Hash] }
        lt        { "< <"   => [ TokenKind::Lt, TokenKind::Lt] }
        lteq      { "<= <=" => [ TokenKind::LtEq, TokenKind::LtEq] }
        ltlt      { "<< <<" => [ TokenKind::LtLt, TokenKind::LtLt] }
        ltlteq    { "<<= <<=" => [ TokenKind::LtLtEq, TokenKind::LtLtEq] }
        minus     { "- -"   => [ TokenKind::Minus, TokenKind::Minus] }
        minuseq   { "-= -=" => [ TokenKind::MinusEq, TokenKind::MinusEq] }
        plus      { "+ +"   => [ TokenKind::Plus, TokenKind::Plus] }
        pluseq    { "+= +=" => [ TokenKind::PlusEq, TokenKind::PlusEq] }
        question  { "? ?"   => [ TokenKind::Question, TokenKind::Question] }
        rem       { "% %"   => [ TokenKind::Rem, TokenKind::Rem] }
        remeq     { "%= %=" => [ TokenKind::RemEq, TokenKind::RemEq] }
        semi      { "; ;"   => [ TokenKind::Semi, TokenKind::Semi] }
        slash     { "/ /"   => [ TokenKind::Slash, TokenKind::Slash] }
        slasheq   { "/= /=" => [ TokenKind::SlashEq, TokenKind::SlashEq] }
        star      { "* *"   => [ TokenKind::Star, TokenKind::Star] }
        stareq    { "*= *=" => [ TokenKind::StarEq, TokenKind::StarEq] }
        tilde     { "~ ~"   => [ TokenKind::Tilde, TokenKind::Tilde] }
        xor       { "^ ^"   => [ TokenKind::Xor, TokenKind::Xor] }
        xoreq     { "^= ^=" => [ TokenKind::XorEq, TokenKind::XorEq] }
        xorxor    { "^^ ^^" => [ TokenKind::XorXor, TokenKind::XorXor] }
    }
}
