//! Prompts the user, reads the lines. Not much more to it than that.
//!
//! This module is in charge of parsing keyboard input and interpreting it for the line editor.

use crate::{editor::Editor, error::*, iter::*, raw::raw};
use std::{
    collections::VecDeque,
    io::{stdout, Bytes, Read, Result, Write},
};

/// Prompts the user, reads the lines. Not much more to it than that.
#[derive(Debug)]
pub struct Repline<'a, R: Read> {
    input: Chars<Flatten<Result<u8>, Bytes<R>>>,

    history: VecDeque<String>, // previous lines
    hindex: usize,             // current index into the history buffer

    ed: Editor<'a>, // the current line buffer
}

impl<'a> Repline<'a, std::io::Stdin> {
    pub fn new(color: &'a str, begin: &'a str, again: &'a str) -> Self {
        Self::with_input(std::io::stdin(), color, begin, again)
    }
}

impl<'a, R: Read> Repline<'a, R> {
    /// Constructs a [Repline] with the given [Reader](Read), color, begin, and again prompts.
    pub fn with_input(input: R, color: &'a str, begin: &'a str, again: &'a str) -> Self {
        Self {
            input: Chars(Flatten(input.bytes())),
            history: Default::default(),
            hindex: 0,
            ed: Editor::new(color, begin, again),
        }
    }
    /// Set the terminal prompt color
    pub fn set_color(&mut self, color: &'a str) {
        self.ed.color = color
    }
    /// Append line to history and clear it
    pub fn accept(&mut self) {
        self.history_append(self.ed.to_string());
        self.ed.clear();
        self.hindex = self.history.len();
    }
    /// Clear the line
    pub fn deny(&mut self) {
        self.ed.clear()
    }
    /// Reads in a line, and returns it for validation
    pub fn read(&mut self) -> ReplResult<String> {
        const INDENT: &str = "    ";
        let mut stdout = stdout().lock();
        let stdout = &mut stdout;
        let _make_raw = raw();
        // self.ed.begin_frame(stdout)?;
        // self.ed.redraw_frame(stdout)?;
        self.ed.print_head(stdout)?;
        loop {
            stdout.flush()?;
            match self.input.next().ok_or(Error::EndOfInput)?? {
                // Ctrl+C: End of Text. Immediately exits.
                '\x03' => {
                    drop(_make_raw);
                    writeln!(stdout)?;
                    return Err(Error::CtrlC(self.ed.to_string()));
                }
                // Ctrl+D: End of Transmission. Ends the current line.
                '\x04' => {
                    drop(_make_raw);
                    writeln!(stdout)?;
                    return Err(Error::CtrlD(self.ed.to_string()));
                }
                // Tab: extend line by 4 spaces
                '\t' => {
                    self.ed.extend(INDENT.chars(), stdout)?;
                }
                // ignore newlines, process line feeds. Not sure how cross-platform this is.
                '\n' => {}
                '\r' => {
                    self.ed.push('\n', stdout)?;
                    return Ok(self.ed.to_string());
                }
                // Ctrl+Backspace in my terminal
                '\x17' => {
                    self.ed.erase_word(stdout)?;
                }
                // Escape sequence
                '\x1b' => self.escape(stdout)?,
                // backspace
                '\x08' | '\x7f' => {
                    let ed = &mut self.ed;
                    if ed.ends_with(INDENT.chars()) {
                        for _ in 0..INDENT.len() {
                            ed.pop(stdout)?;
                        }
                    } else {
                        ed.pop(stdout)?;
                    }
                }
                c if c.is_ascii_control() => {
                    if cfg!(debug_assertions) {
                        self.ed.extend(c.escape_debug(), stdout)?;
                    }
                }
                c => {
                    self.ed.push(c, stdout)?;
                }
            }
        }
    }
    /// Handle ANSI Escape
    fn escape<W: Write>(&mut self, w: &mut W) -> ReplResult<()> {
        match self.input.next().ok_or(Error::EndOfInput)?? {
            '[' => self.csi(w)?,
            'O' => todo!("Process alternate character mode"),
            other => self.ed.extend(other.escape_debug(), w)?,
        }
        Ok(())
    }
    /// Handle ANSI Control Sequence Introducer
    fn csi<W: Write>(&mut self, w: &mut W) -> ReplResult<()> {
        match self.input.next().ok_or(Error::EndOfInput)?? {
            'A' => {
                self.hindex = self.hindex.saturating_sub(1);
                self.restore_history(w)?
            }
            'B' => {
                self.hindex = self.hindex.saturating_add(1).min(self.history.len());
                self.restore_history(w)?
            }
            'C' => self.ed.cursor_forward(1, w)?,
            'D' => self.ed.cursor_back(1, w)?,
            'H' => self.ed.home(w)?,
            'F' => self.ed.end(w)?,
            '3' => {
                if let '~' = self.input.next().ok_or(Error::EndOfInput)?? {
                    let _ = self.ed.delete(w);
                }
            }
            other => {
                if cfg!(debug_assertions) {
                    self.ed.extend(other.escape_debug(), w)?;
                }
            }
        }
        Ok(())
    }
    /// Restores the currently selected history
    fn restore_history<W: Write>(&mut self, w: &mut W) -> ReplResult<()> {
        let Self { history, hindex, ed, .. } = self;
        ed.undraw(w)?;
        ed.clear();
        ed.print_head(w)?;
        if let Some(history) = history.get(*hindex) {
            ed.extend(history.chars(), w)?
        }
        Ok(())
    }

    /// Append line to history
    fn history_append(&mut self, mut buf: String) {
        while buf.ends_with(char::is_whitespace) {
            buf.pop();
        }
        if !self.history.contains(&buf) {
            self.history.push_back(buf)
        }
        while self.history.len() > 20 {
            self.history.pop_front();
        }
    }
}
