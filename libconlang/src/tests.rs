mod token {
    // TODO
}
mod ast {
    // TODO
}
mod lexer {
    #[allow(unused_imports)]
    use crate::{
        lexer::Lexer,
        token::{Token, TokenData, Keyword, Type},
    };

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
            [ $(Type::Keyword(Keyword::$k),)* ]
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
            l_curly   { "{ {"   => [ Type::LCurly, Type::LCurly ] }
            r_curly   { "} }"   => [ Type::RCurly, Type::RCurly ] }
            l_brack   { "[ ["   => [ Type::LBrack, Type::LBrack ] }
            r_brack   { "] ]"   => [ Type::RBrack, Type::RBrack ] }
            l_paren   { "( ("   => [ Type::LParen, Type::LParen ] }
            r_paren   { ") )"   => [ Type::RParen, Type::RParen ] }
            amp       { "& &"   => [ Type::Amp, Type::Amp ] }
            amp_amp   { "&& &&" => [ Type::AmpAmp, Type::AmpAmp ] }
            amp_eq    { "&= &=" => [ Type::AmpEq, Type::AmpEq ] }
            arrow     { "-> ->" => [ Type::Arrow, Type::Arrow] }
            at        { "@ @"   => [ Type::At, Type::At] }
            backslash { "\\ \\" => [ Type::Backslash, Type::Backslash] }
            bang      { "! !"   => [ Type::Bang, Type::Bang] }
            bangbang  { "!! !!" => [ Type::BangBang, Type::BangBang] }
            bangeq    { "!= !=" => [ Type::BangEq, Type::BangEq] }
            bar       { "| |"   => [ Type::Bar, Type::Bar] }
            barbar    { "|| ||" => [ Type::BarBar, Type::BarBar] }
            bareq     { "|= |=" => [ Type::BarEq, Type::BarEq] }
            colon     { ": :"   => [ Type::Colon, Type::Colon] }
            comma     { ", ,"   => [ Type::Comma, Type::Comma] }
            dot       { ". ."   => [ Type::Dot, Type::Dot] }
            dotdot    { ".. .." => [ Type::DotDot, Type::DotDot] }
            dotdoteq  { "..= ..=" => [ Type::DotDotEq, Type::DotDotEq] }
            eq        { "= ="   => [ Type::Eq, Type::Eq] }
            eqeq      { "== ==" => [ Type::EqEq, Type::EqEq] }
            fatarrow  { "=> =>" => [ Type::FatArrow, Type::FatArrow] }
            grave     { "` `"   => [ Type::Grave, Type::Grave] }
            gt        { "> >"   => [ Type::Gt, Type::Gt] }
            gteq      { ">= >=" => [ Type::GtEq, Type::GtEq] }
            gtgt      { ">> >>" => [ Type::GtGt, Type::GtGt] }
            gtgteq    { ">>= >>=" => [ Type::GtGtEq, Type::GtGtEq] }
            hash      { "# #"   => [ Type::Hash, Type::Hash] }
            lt        { "< <"   => [ Type::Lt, Type::Lt] }
            lteq      { "<= <=" => [ Type::LtEq, Type::LtEq] }
            ltlt      { "<< <<" => [ Type::LtLt, Type::LtLt] }
            ltlteq    { "<<= <<=" => [ Type::LtLtEq, Type::LtLtEq] }
            minus     { "- -"   => [ Type::Minus, Type::Minus] }
            minuseq   { "-= -=" => [ Type::MinusEq, Type::MinusEq] }
            plus      { "+ +"   => [ Type::Plus, Type::Plus] }
            pluseq    { "+= +=" => [ Type::PlusEq, Type::PlusEq] }
            question  { "? ?"   => [ Type::Question, Type::Question] }
            rem       { "% %"   => [ Type::Rem, Type::Rem] }
            remeq     { "%= %=" => [ Type::RemEq, Type::RemEq] }
            semi      { "; ;"   => [ Type::Semi, Type::Semi] }
            slash     { "/ /"   => [ Type::Slash, Type::Slash] }
            slasheq   { "/= /=" => [ Type::SlashEq, Type::SlashEq] }
            star      { "* *"   => [ Type::Star, Type::Star] }
            stareq    { "*= *=" => [ Type::StarEq, Type::StarEq] }
            tilde     { "~ ~"   => [ Type::Tilde, Type::Tilde] }
            xor       { "^ ^"   => [ Type::Xor, Type::Xor] }
            xoreq     { "^= ^=" => [ Type::XorEq, Type::XorEq] }
            xorxor    { "^^ ^^" => [ Type::XorXor, Type::XorXor] }
        }
    }
}
mod parser {
    // TODO
}
mod interpreter {
    // TODO
}
