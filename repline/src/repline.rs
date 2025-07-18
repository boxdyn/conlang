//! Prompts the user, reads the lines. Not much more to it than that.
//!
//! This module is in charge of parsing keyboard input and interpreting it for the line editor.
#![allow(clippy::unbuffered_bytes)]

use crate::{editor::Editor, error::*, iter::*, raw::raw};
use std::{
    collections::VecDeque,
    io::{Bytes, Read, Result, Write, stdout},
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
    pub fn swap_input<S: Read>(self, new_input: S) -> Repline<'a, S> {
        Repline {
            input: Chars(Flatten(new_input.bytes())),
            history: self.history,
            hindex: self.hindex,
            ed: self.ed,
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
                '\t' => self.ed.extend(INDENT.chars(), stdout)?,
                // ignore newlines, process line feeds. Not sure how cross-platform this is.
                '\n' => {}
                '\r' => {
                    self.ed.push('\n', stdout)?;
                    if self.ed.at_end() {
                        return Ok(self.ed.to_string());
                    }
                }
                // Ctrl+Backspace in my terminal
                '\x17' => self.ed.erase_word(stdout)?,
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
    /// Prints a message without moving the cursor
    pub fn print_inline(&mut self, value: impl std::fmt::Display) -> ReplResult<()> {
        let mut stdout = stdout().lock();
        self.print_err(&mut stdout, value)
    }
    /// Prints a message (ideally an error) without moving the cursor
    fn print_err<W: Write>(&self, w: &mut W, value: impl std::fmt::Display) -> ReplResult<()> {
        self.ed.print_err(value, w)
    }
    // Prints some debug info into the editor's buffer and the provided writer
    pub fn put<D: std::fmt::Display, W: Write>(&mut self, disp: D, w: &mut W) -> ReplResult<()> {
        self.ed.extend(format!("{disp}").chars(), w)
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
            'A' if self.ed.at_start() && self.hindex > 0 => {
                self.hindex -= 1;
                self.restore_history(w)?;
            }
            'A' => self.ed.cursor_up(w)?,
            'B' if self.ed.at_end() && self.hindex < self.history.len().saturating_sub(1) => {
                self.hindex += 1;
                self.restore_history(w)?;
            }
            'B' => self.ed.cursor_down(w)?,
            'C' => self.ed.cursor_forward(w)?,
            'D' => self.ed.cursor_back(w)?,
            'H' => self.ed.cursor_line_start(w)?,
            'F' => self.ed.cursor_line_end(w)?,
            '3' => {
                if let '~' = self.input.next().ok_or(Error::EndOfInput)?? {
                    self.ed.delete(w)?;
                }
            }
            '5' => {
                if let '~' = self.input.next().ok_or(Error::EndOfInput)?? {
                    self.ed.cursor_start(w)?
                }
            }
            '6' => {
                if let '~' = self.input.next().ok_or(Error::EndOfInput)?? {
                    self.ed.cursor_end(w)?
                }
            }
            other => {
                if cfg!(debug_assertions) {
                    self.print_err(w, other.escape_debug())?;
                }
            }
        }
        Ok(())
    }
    /// Restores the currently selected history
    fn restore_history<W: Write>(&mut self, w: &mut W) -> ReplResult<()> {
        let Self { history, hindex, ed, .. } = self;
        if let Some(history) = history.get(*hindex) {
            ed.restore(history, w)?;
            ed.print_err(
                format_args!("\t\x1b[30mHistory {hindex} restored!\x1b[0m"),
                w,
            )?;
        }
        Ok(())
    }

    /// Append line to history
    fn history_append(&mut self, mut buf: String) {
        while buf.ends_with(char::is_whitespace) {
            buf.pop();
        }
        if let Some(idx) = self.history.iter().position(|v| *v == buf) {
            self.history
                .remove(idx)
                .expect("should have just found this");
        };
        self.history.push_back(buf);
        while self.history.len() > 20 {
            self.history.pop_front();
        }
    }
}
