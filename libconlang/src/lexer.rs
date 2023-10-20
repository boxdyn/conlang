//! Converts a text file into tokens
use crate::token::{Token, Type};
use lerox::Combinator;

pub struct IntoIter<'t> {
    lexer: Lexer<'t>,
}
impl<'t> Iterator for IntoIter<'t> {
    type Item = Token;
    fn next(&mut self) -> Option<Self::Item> {
        self.lexer.any()
    }
}
impl<'t> IntoIterator for Lexer<'t> {
    type Item = Token;
    type IntoIter = IntoIter<'t>;
    fn into_iter(self) -> Self::IntoIter {
        IntoIter { lexer: self }
    }
}

#[derive(Clone, Debug)]
pub struct Lexer<'t> {
    text: &'t str,
    cursor: usize,
    line: u32,
    col: u32,
}
/// Implements the non-terminals of a language
impl<'t> Lexer<'t> {
    pub fn new(text: &'t str) -> Self {
        Self { text, cursor: 0, line: 1, col: 1 }
    }
    /// Consumes the entire [`Lexer`], producing a [`Vec<Token>`]
    /// and returning the original string
    pub fn consume(self) -> (Vec<Token>, &'t str) {
        let text = self.text;
        (self.into_iter().collect(), text)
    }
    /// Counts some length
    #[inline]
    fn count_len(&mut self, len: usize) -> &mut Self {
        self.cursor += len;
        self.col += len as u32;
        self
    }
    /// Counts a line
    #[inline]
    fn count_line(&mut self, lines: u32) -> &mut Self {
        self.line += lines;
        self.col = 1;
        self
    }
    /// Skips whitespace in the text
    fn skip_whitespace(&mut self) {
        self.count_len(
            Rule::new(self.text())
                .and_any(Rule::whitespace_not_newline)
                .end()
                .unwrap_or_default(),
        );
        if Rule::new(self.text()).char('\n').end().is_some() {
            // recurse until all newlines are skipped
            self.count_len(1).count_line(1).skip_whitespace();
        }
    }
    /// Advances the cursor and produces a token from a provided [Rule] function
    fn map_rule<F>(&mut self, rule: F, ty: Type) -> Option<Token>
    where F: Fn(Rule) -> Rule {
        self.skip_whitespace();
        let (line, col, start) = (self.line, self.col, self.cursor);
        self.count_len(Rule::new(self.text()).and(rule).end()?);
        Some(Token::new(ty, start, self.cursor, line, col))
    }
    /// Gets a slice of text beginning at the cursor
    fn text(&self) -> &str {
        &self.text[self.cursor..]
    }
    // classifies a single arbitrary token
    /// Returns the result of the rule with the highest precedence, if any matches
    pub fn any(&mut self) -> Option<Token> {
        None.or_else(|| self.comment())
            .or_else(|| self.identifier())
            .or_else(|| self.literal())
            .or_else(|| self.delimiter())
            .or_else(|| self.punctuation())
            .or_else(|| self.invalid())
    }
    /// Attempts to produce a [Type::String], [Type::Float], or [Type::Integer]
    pub fn literal(&mut self) -> Option<Token> {
        None.or_else(|| self.string())
            .or_else(|| self.character())
            .or_else(|| self.float())
            .or_else(|| self.integer())
    }
    /// Evaluates delimiter rules
    pub fn delimiter(&mut self) -> Option<Token> {
        None.or_else(|| self.l_brack())
            .or_else(|| self.r_brack())
            .or_else(|| self.l_curly())
            .or_else(|| self.r_curly())
            .or_else(|| self.l_paren())
            .or_else(|| self.r_paren())
    }
    /// Evaluates punctuation rules
    pub fn punctuation(&mut self) -> Option<Token> {
        None.or_else(|| self.amp_amp()) //      &&
            .or_else(|| self.amp_eq()) //       &=
            .or_else(|| self.amp()) //          &
            .or_else(|| self.at()) //           @
            .or_else(|| self.backslash()) //    \
            .or_else(|| self.bang_bang()) //    !!
            .or_else(|| self.bang_eq()) //      !=
            .or_else(|| self.bang()) //         !
            .or_else(|| self.bar_bar()) //      ||
            .or_else(|| self.bar_eq()) //       |=
            .or_else(|| self.bar()) //          |
            .or_else(|| self.colon()) //        :
            .or_else(|| self.comma()) //        ,
            .or_else(|| self.dot_dot_eq()) //   ..=
            .or_else(|| self.dot_dot()) //      ..
            .or_else(|| self.dot()) //          .
            .or_else(|| self.eq_eq()) //        ==
            .or_else(|| self.fatarrow()) //     =>
            .or_else(|| self.eq()) //           =
            .or_else(|| self.grave()) //        `
            .or_else(|| self.gt_eq()) //        >=
            .or_else(|| self.gt_gt_eq()) //     >>=
            .or_else(|| self.gt_gt()) //        >>
            .or_else(|| self.gt()) //           >
            .or_else(|| self.hash()) //         #
            .or_else(|| self.lt_eq()) //        <=
            .or_else(|| self.lt_lt_eq()) //     <<=
            .or_else(|| self.lt_lt()) //        <<
            .or_else(|| self.lt()) //           <
            .or_else(|| self.minus_eq()) //     -=
            .or_else(|| self.arrow()) //        ->
            .or_else(|| self.minus()) //        -
            .or_else(|| self.plus_eq()) //      +=
            .or_else(|| self.plus()) //         +
            .or_else(|| self.question()) //     ?
            .or_else(|| self.rem_eq()) //       %=
            .or_else(|| self.rem()) //          %
            .or_else(|| self.semi()) //         ;
            .or_else(|| self.slash_eq()) //     /=
            .or_else(|| self.slash()) //        /
            .or_else(|| self.star_eq()) //      *=
            .or_else(|| self.star()) //         *
            .or_else(|| self.tilde()) //        ~
            .or_else(|| self.xor_eq()) //       ^=
            .or_else(|| self.xor_xor()) //      ^^
            .or_else(|| self.xor()) //          ^
    }
    pub fn unary_op(&mut self) -> Option<Token> {
        self.bang().or_else(|| self.minus())
    }
    // functions for lexing individual tokens
    pub fn invalid(&mut self) -> Option<Token> {
        self.map_rule(|r| r.invalid(), Type::Invalid)
    }
    // comments
    pub fn comment(&mut self) -> Option<Token> {
        self.map_rule(|r| r.comment(), Type::Comment)
    }
    // identifiers
    pub fn identifier(&mut self) -> Option<Token> {
        self.map_rule(|r| r.identifier(), Type::Identifier)
            .map(|token| match self.text[token.range()].parse() {
                Ok(kw) => token.cast(Type::Keyword(kw)),
                Err(_) => token,
            })
    }
    // literals
    pub fn integer(&mut self) -> Option<Token> {
        self.map_rule(|r| r.integer(), Type::Integer)
    }
    pub fn float(&mut self) -> Option<Token> {
        self.map_rule(|r| r.float(), Type::Float)
    }
    pub fn string(&mut self) -> Option<Token> {
        // TODO: count lines and columns properly within string
        self.map_rule(|r| r.string(), Type::String)
            .map(|t| t.rebound(t.head + 1, t.tail - 1))
    }
    pub fn character(&mut self) -> Option<Token> {
        self.map_rule(|r| r.character(), Type::Character)
            .map(|t| t.rebound(t.head + 1, t.tail - 1))
    }
    // delimiters
    pub fn l_brack(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char('['), Type::LBrack)
    }
    pub fn r_brack(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char(']'), Type::RBrack)
    }
    pub fn l_curly(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char('{'), Type::LCurly)
    }
    pub fn r_curly(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char('}'), Type::RCurly)
    }
    pub fn l_paren(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char('('), Type::LParen)
    }
    pub fn r_paren(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char(')'), Type::RParen)
    }
    // compound punctuation
    pub fn lt_lt(&mut self) -> Option<Token> {
        self.map_rule(|r| r.str("<<"), Type::LtLt)
    }
    pub fn gt_gt(&mut self) -> Option<Token> {
        self.map_rule(|r| r.str(">>"), Type::GtGt)
    }
    pub fn amp_amp(&mut self) -> Option<Token> {
        self.map_rule(|r| r.str("&&"), Type::AmpAmp)
    }
    pub fn bar_bar(&mut self) -> Option<Token> {
        self.map_rule(|r| r.str("||"), Type::BarBar)
    }
    pub fn bang_bang(&mut self) -> Option<Token> {
        self.map_rule(|r| r.str("!!"), Type::BangBang)
    }
    pub fn xor_xor(&mut self) -> Option<Token> {
        self.map_rule(|r| r.str("^^"), Type::XorXor)
    }
    pub fn eq_eq(&mut self) -> Option<Token> {
        self.map_rule(|r| r.str("=="), Type::EqEq)
    }
    pub fn gt_eq(&mut self) -> Option<Token> {
        self.map_rule(|r| r.str(">="), Type::GtEq)
    }
    pub fn lt_eq(&mut self) -> Option<Token> {
        self.map_rule(|r| r.str("<="), Type::LtEq)
    }
    pub fn bang_eq(&mut self) -> Option<Token> {
        self.map_rule(|r| r.str("!="), Type::BangEq)
    }
    pub fn star_eq(&mut self) -> Option<Token> {
        self.map_rule(|r| r.str("*="), Type::StarEq)
    }
    pub fn slash_eq(&mut self) -> Option<Token> {
        self.map_rule(|r| r.str("/="), Type::SlashEq)
    }
    pub fn rem_eq(&mut self) -> Option<Token> {
        self.map_rule(|r| r.str("%="), Type::RemEq)
    }
    pub fn plus_eq(&mut self) -> Option<Token> {
        self.map_rule(|r| r.str("+="), Type::PlusEq)
    }
    pub fn minus_eq(&mut self) -> Option<Token> {
        self.map_rule(|r| r.str("-="), Type::MinusEq)
    }
    pub fn amp_eq(&mut self) -> Option<Token> {
        self.map_rule(|r| r.str("&="), Type::AmpEq)
    }
    pub fn bar_eq(&mut self) -> Option<Token> {
        self.map_rule(|r| r.str("|="), Type::BarEq)
    }
    pub fn xor_eq(&mut self) -> Option<Token> {
        self.map_rule(|r| r.str("^="), Type::XorEq)
    }
    pub fn lt_lt_eq(&mut self) -> Option<Token> {
        self.map_rule(|r| r.str("<<="), Type::LtLtEq)
    }
    pub fn gt_gt_eq(&mut self) -> Option<Token> {
        self.map_rule(|r| r.str(">>="), Type::GtGtEq)
    }
    pub fn dot_dot_eq(&mut self) -> Option<Token> {
        self.map_rule(|r| r.str("..="), Type::DotDotEq)
    }
    pub fn dot_dot(&mut self) -> Option<Token> {
        self.map_rule(|r| r.str(".."), Type::DotDot)
    }
    pub fn arrow(&mut self) -> Option<Token> {
        self.map_rule(|r| r.str("->"), Type::Arrow)
    }
    pub fn fatarrow(&mut self) -> Option<Token> {
        self.map_rule(|r| r.str("=>"), Type::FatArrow)
    }
    // simple punctuation
    pub fn semi(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char(';'), Type::Semi)
    }
    pub fn dot(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char('.'), Type::Dot)
    }
    pub fn star(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char('*'), Type::Star)
    }
    pub fn slash(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char('/'), Type::Slash)
    }
    pub fn plus(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char('+'), Type::Plus)
    }
    pub fn minus(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char('-'), Type::Minus)
    }
    pub fn rem(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char('%'), Type::Rem)
    }
    pub fn bang(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char('!'), Type::Bang)
    }
    pub fn eq(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char('='), Type::Eq)
    }
    pub fn lt(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char('<'), Type::Lt)
    }
    pub fn gt(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char('>'), Type::Gt)
    }
    pub fn amp(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char('&'), Type::Amp)
    }
    pub fn bar(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char('|'), Type::Bar)
    }
    pub fn xor(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char('^'), Type::Xor)
    }
    pub fn hash(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char('#'), Type::Hash)
    }
    pub fn at(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char('@'), Type::At)
    }
    pub fn colon(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char(':'), Type::Colon)
    }
    pub fn question(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char('?'), Type::Question)
    }
    pub fn comma(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char(','), Type::Comma)
    }
    pub fn tilde(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char('~'), Type::Tilde)
    }
    pub fn grave(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char('`'), Type::Grave)
    }
    pub fn backslash(&mut self) -> Option<Token> {
        self.map_rule(|r| r.char('\\'), Type::Backslash)
    }
}

// TODO: use real, functional parser-combinators here to produce tokens
/// A lexer [Rule] matches patterns in text in a declarative manner
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule<'t> {
    text: &'t str,
    taken: usize,
    is_alright: bool,
}
impl<'t> Rule<'t> {
    pub fn new(text: &'t str) -> Self {
        Self { text, taken: 0, is_alright: true }
    }
    pub fn end(self) -> Option<usize> {
        self.is_alright.then_some(self.taken)
    }
    pub fn remaining(&self) -> &str {
        self.text
    }
}

impl<'t> Rule<'t> {
    /// Matches any sequence of non-whitespace characters
    pub fn invalid(self) -> Self {
        self.and_many(Self::not_whitespace)
    }
    /// Matches a block, line, or shebang comment
    pub fn comment(self) -> Self {
        self.and_either(Self::line_comment, Self::block_comment)
    }
    /// Matches a line or shebang comment
    fn line_comment(self) -> Self {
        // line_comment := ("//" | "#!/") (!newline)*
        self.str("//")
            .or(|r| r.str("#!/"))
            .and_any(|r| r.not_char('\n'))
    }
    /// Matches a block comment
    fn block_comment(self) -> Self {
        // block_comment := "/*" (block_comment | all_but("*/"))* "*/"
        self.str("/*")
            .and_any(|r| r.and_either(|f| f.block_comment(), |g| g.not_str("*/")))
            .str("*/")
    }
    /// Matches a Rust-style identifier
    pub fn identifier(self) -> Self {
        // identifier := ('_' | XID_START) ~ XID_CONTINUE*
        self.char('_')
            .or(Rule::xid_start)
            .and_any(Rule::xid_continue)
    }
    /// Matches a Rust-style base-prefixed int literal
    fn integer_kind(self, prefix: &str, digit: impl Fn(Self) -> Self) -> Self {
        // int_kind<Prefix, Digit> := Prefix '_'* Digit (Digit | '_')*
        self.str(prefix)
            .and_any(|r| r.char('_'))
            .and(&digit)
            .and_any(|r| r.and(&digit).or(|r| r.char('_')))
    }
    /// Matches a Rust-style integer literal
    pub fn integer(self) -> Self {
        // integer = (int_kind<0d, dec_digit> | int_kind<0x, hex_digit>
        //           | int_kind<0o, oct_digit> | int_kind<0b, bin_digit> | dec_digit (dec_digit | '_')*)
        self.and_one_of(&[
            &|rule| rule.integer_kind("0d", Rule::dec_digit),
            &|rule| rule.integer_kind("0x", Rule::hex_digit),
            &|rule| rule.integer_kind("0o", Rule::oct_digit),
            &|rule| rule.integer_kind("0b", Rule::bin_digit),
            &|rule| {
                rule.dec_digit()
                    .and_any(|r| r.dec_digit().or(|r| r.char('_')))
            },
        ])
    }
    /// Matches a float literal
    // TODO: exponent form
    pub fn float(self) -> Self {
        self.and_any(Rule::dec_digit)
            .char('.')
            .and_many(Rule::dec_digit)
    }
    /// Matches one apostrophe-delimited char literal
    pub fn character(self) -> Self {
        self.char('\'').character_continue().char('\'')
    }
    pub fn character_continue(self) -> Self {
        self.and(|rule| rule.string_escape().or(|rule| rule.not_char('\'')))
    }
    /// Matches one quote-delimited string literal
    pub fn string(self) -> Self {
        self.char('"').and_any(Rule::string_continue).char('"')
    }
    /// Matches one string escape sequence or non-`"` characcter
    pub fn string_continue(self) -> Self {
        self.and(Rule::string_escape).or(|rule| rule.not_char('"'))
    }
}

impl<'t> Rule<'t> {
    /// Matches a char lexicographically between start and end
    pub fn char_between(self, start: char, end: char) -> Self {
        self.char_fn(|c| start <= c && c <= end)
    }
    /// Matches a single char
    pub fn char(self, c: char) -> Self {
        self.has(|rule| rule.text.starts_with(c), 1)
    }
    /// Matches the entirety of a string slice
    pub fn str(self, s: &str) -> Self {
        self.has(|rule| rule.text.starts_with(s), s.len())
    }
    /// Matches a char based on the output of a function
    pub fn char_fn(self, f: impl Fn(char) -> bool) -> Self {
        self.and(|rule| match rule.text.strip_prefix(&f) {
            Some(text) => Self { text, taken: rule.taken + next_utf8(rule.text, 1), ..rule },
            None => Self { is_alright: false, ..rule },
        })
    }
    /// Matches a single char except c
    pub fn not_char(self, c: char) -> Self {
        self.has(|rule| !rule.text.starts_with(c), 1)
    }
    /// Matches a single char unless the text starts with s
    pub fn not_str(self, s: &str) -> Self {
        self.has(|rule| !rule.text.starts_with(s), 1)
    }
    // commonly used character classes
    /// Matches one of any character
    pub fn any(self) -> Self {
        self.has(|_| true, 1)
    }
    /// Matches one whitespace
    pub fn whitespace(self) -> Self {
        self.char_fn(|c| c.is_whitespace())
    }
    /// Matches one whitespace, except `'\n'`
    pub fn whitespace_not_newline(self) -> Self {
        self.char_fn(|c| '\n' != c && c.is_whitespace())
    }
    /// Matches anything but whitespace
    pub fn not_whitespace(self) -> Self {
        self.char_fn(|c| !c.is_whitespace())
    }
    /// Matches one XID_START
    pub fn xid_start(self) -> Self {
        use unicode_xid::UnicodeXID;
        self.char_fn(UnicodeXID::is_xid_start)
    }
    /// Matches one XID_CONTINUE
    pub fn xid_continue(self) -> Self {
        use unicode_xid::UnicodeXID;
        self.char_fn(UnicodeXID::is_xid_continue)
    }
    /// Matches one hexadecimal digit
    pub fn hex_digit(self) -> Self {
        self.char_fn(|c| c.is_ascii_hexdigit())
    }
    /// Matches one decimal digit
    pub fn dec_digit(self) -> Self {
        self.char_fn(|c| c.is_ascii_digit())
    }
    /// Matches one octal digit
    pub fn oct_digit(self) -> Self {
        self.char_between('0', '7')
    }
    /// Matches one binary digit
    pub fn bin_digit(self) -> Self {
        self.char_between('0', '1')
    }
    /// Matches any string escape "\."
    pub fn string_escape(self) -> Self {
        self.char('\\').and(Rule::any)
    }
    /// Performs a consuming condition assertion on the input
    fn has(self, condition: impl Fn(&Self) -> bool, len: usize) -> Self {
        let len = next_utf8(self.text, len);
        self.and(|rule| match condition(&rule) && !rule.text.is_empty() {
            true => Self { text: &rule.text[len..], taken: rule.taken + len, ..rule },
            false => Self { is_alright: false, ..rule },
        })
    }
}

impl<'t> lerox::Combinator for Rule<'t> {
    fn is_alright(&self) -> bool {
        self.is_alright
    }
    fn into_alright(self) -> Self {
        Self { is_alright: true, ..self }
    }
}

/// Returns the index of the next unicode character, rounded up
fn next_utf8(text: &str, mut index: usize) -> usize {
    index = index.min(text.len());
    while !text.is_char_boundary(index) {
        index += 1
    }
    index
}
