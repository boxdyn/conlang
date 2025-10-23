//! The [Editor] is a multi-line buffer of [`char`]s which operates on an ANSI-compatible terminal.

use crossterm::{cursor::*, queue, style::*, terminal::*};
use std::{collections::VecDeque, fmt::Display, io::Write};

use super::error::ReplResult;

fn is_newline(c: &char) -> bool {
    *c == '\n'
}

fn write_chars<'a, W: Write>(
    c: impl IntoIterator<Item = &'a char>,
    w: &mut W,
) -> std::io::Result<()> {
    for c in c {
        queue!(w, Print(c))?;
    }
    Ok(())
}

/// A multi-line editor which operates on an un-cleared ANSI terminal.
#[derive(Clone, Debug)]
pub struct Editor<'a> {
    head: VecDeque<char>,
    tail: VecDeque<char>,

    pub color: &'a str,
    pub begin: &'a str,
    pub again: &'a str,
}

impl<'a> Editor<'a> {
    /// Constructs a new Editor with the provided prompt color, begin prompt, and again prompt.
    pub fn new(color: &'a str, begin: &'a str, again: &'a str) -> Self {
        Self { head: Default::default(), tail: Default::default(), color, begin, again }
    }

    /// Returns an iterator over characters in the editor.
    pub fn iter(&self) -> impl Iterator<Item = &char> {
        let Self { head, tail, .. } = self;
        head.iter().chain(tail.iter())
    }

    fn putchar<W: Write>(&self, c: char, w: &mut W) -> ReplResult<()> {
        let Self { color, again, .. } = self;
        match c {
            '\n' => queue!(
                w,
                Print('\n'),
                MoveToColumn(0),
                Print(color),
                Print(again),
                Print(ResetColor),
                Print(' ')
            ),
            c => queue!(w, Print(c)),
        }?;
        Ok(())
    }

    pub fn redraw_head<W: Write>(&self, w: &mut W) -> ReplResult<()> {
        let Self { head, color, begin, .. } = self;
        match head.iter().copied().filter(is_newline).count() {
            0 => queue!(w, MoveToColumn(0)),
            n => queue!(w, MoveUp(n as u16)),
        }?;

        queue!(w, Print(color), Print(begin), Print(ResetColor), Print(' '))?;
        for c in head {
            self.putchar(*c, w)?;
        }
        Ok(())
    }

    pub fn redraw_tail<W: Write>(&self, w: &mut W) -> ReplResult<()> {
        let Self { tail, .. } = self;
        queue!(w, SavePosition, Clear(ClearType::FromCursorDown))?;
        for c in tail {
            self.putchar(*c, w)?;
        }
        queue!(w, RestorePosition)?;
        Ok(())
    }

    /// Prints the characters before the cursor on the current line.
    pub fn print_head<W: Write>(&self, w: &mut W) -> ReplResult<()> {
        let Self { head, color, begin, again, .. } = self;
        let nl = self.head.iter().rposition(is_newline).map(|n| n + 1);
        let prompt = if nl.is_some() { again } else { begin };

        queue!(
            w,
            MoveToColumn(0),
            Print(color),
            Print(prompt),
            ResetColor,
            Print(' '),
        )?;

        write_chars(head.iter().skip(nl.unwrap_or(0)), w)?;
        Ok(())
    }

    /// Prints the characters after the cursor on the current line.
    pub fn print_tail<W: Write>(&self, w: &mut W) -> ReplResult<()> {
        let Self { tail, .. } = self;
        queue!(w, SavePosition, Clear(ClearType::UntilNewLine))?;
        write_chars(tail.iter().take_while(|&c| !is_newline(c)), w)?;
        queue!(w, RestorePosition)?;
        Ok(())
    }

    pub fn print_err<W: Write>(&self, err: impl Display, w: &mut W) -> ReplResult<()> {
        queue!(
            w,
            SavePosition,
            Clear(ClearType::UntilNewLine),
            Print(err),
            RestorePosition
        )?;
        Ok(())
    }

    /// Writes a character at the cursor, shifting the text around as necessary.
    pub fn push<W: Write>(&mut self, c: char, w: &mut W) -> ReplResult<()> {
        self.head.push_back(c);
        queue!(w, Clear(ClearType::UntilNewLine))?;
        self.putchar(c, w)?;
        match c {
            '\n' => self.redraw_tail(w),
            _ => self.print_tail(w),
        }
    }

    /// Erases a character at the cursor, shifting the text around as necessary.
    pub fn pop<W: Write>(&mut self, w: &mut W) -> ReplResult<Option<char>> {
        let c = self.head.pop_back();

        match c {
            None => return Ok(None),
            Some('\n') => {
                queue!(w, MoveToPreviousLine(1))?;
                self.print_head(w)?;
                self.redraw_tail(w)?;
            }
            Some(_) => {
                queue!(w, MoveLeft(1), Clear(ClearType::UntilNewLine))?;
                self.print_tail(w)?;
            }
        }

        Ok(c)
    }

    /// Pops the character after the cursor, redrawing if necessary
    pub fn delete<W: Write>(&mut self, w: &mut W) -> ReplResult<Option<char>> {
        let c = self.tail.pop_front();
        match c {
            Some('\n') => self.redraw_tail(w)?,
            _ => self.print_tail(w)?,
        }
        Ok(c)
    }

    /// Writes characters into the editor at the location of the cursor.
    pub fn extend<T: IntoIterator<Item = char>, W: Write>(
        &mut self,
        iter: T,
        w: &mut W,
    ) -> ReplResult<()> {
        for c in iter {
            self.push(c, w)?;
        }
        Ok(())
    }

    /// Sets the editor to the contents of a string, placing the cursor at the end.
    pub fn restore<W: Write>(&mut self, s: &str, w: &mut W) -> ReplResult<()> {
        match self.head.iter().copied().filter(is_newline).count() {
            0 => queue!(w, MoveToColumn(0), Clear(ClearType::FromCursorDown))?,
            n => queue!(w, MoveUp(n as u16), Clear(ClearType::FromCursorDown))?,
        };
        self.clear();
        self.print_head(w)?;
        self.extend(s.chars(), w)
    }

    /// Clears the editor, removing all characters.
    pub fn clear(&mut self) {
        self.head.clear();
        self.tail.clear();
    }

    /// Erases a word from the buffer, where a word is any non-whitespace characters
    /// preceded by a single whitespace character
    pub fn erase_word<W: Write>(&mut self, w: &mut W) -> ReplResult<()> {
        while self.pop(w)?.filter(|c| !c.is_whitespace()).is_some() {}
        Ok(())
    }

    /// Returns the number of characters in the buffer
    pub fn len(&self) -> usize {
        self.head.len() + self.tail.len()
    }

    /// Returns true if the cursor is at the start of the buffer
    pub fn at_start(&self) -> bool {
        self.head.is_empty()
    }
    /// Returns true if the cursor is at the end of the buffer
    pub fn at_end(&self) -> bool {
        self.tail.is_empty()
    }

    /// Returns true if the cursor is at the start of a line
    pub fn at_line_start(&self) -> bool {
        matches!(self.head.back(), None | Some('\n'))
    }

    /// Returns true if the cursor is at the end of a line
    pub fn at_line_end(&self) -> bool {
        matches!(self.tail.front(), None | Some('\n'))
    }

    /// Returns true if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.at_start() && self.at_end()
    }

    /// Returns true if the buffer ends with a given pattern
    pub fn ends_with(&self, iter: impl DoubleEndedIterator<Item = char>) -> bool {
        let mut iter = iter.rev();
        let mut head = self.head.iter().rev();
        loop {
            match (iter.next(), head.next()) {
                (None, _) => break true,
                (Some(_), None) => break false,
                (Some(a), Some(b)) if a != *b => break false,
                (Some(_), Some(_)) => continue,
            }
        }
    }

    /// Moves the cursor back `steps` steps
    pub fn cursor_back<W: Write>(&mut self, w: &mut W) -> ReplResult<()> {
        let Some(c) = self.head.pop_back() else {
            return Ok(());
        };

        self.tail.push_front(c);
        match c {
            '\n' => {
                queue!(w, MoveToPreviousLine(1))?;
                self.print_head(w)
            }
            _ => queue!(w, MoveLeft(1)).map_err(Into::into),
        }
    }

    /// Moves the cursor forward `steps` steps
    pub fn cursor_forward<W: Write>(&mut self, w: &mut W) -> ReplResult<()> {
        let Some(c) = self.tail.pop_front() else {
            return Ok(());
        };

        self.head.push_back(c);
        match c {
            '\n' => {
                queue!(w, MoveToNextLine(1))?;
                self.print_head(w)
            }
            _ => queue!(w, MoveRight(1)).map_err(Into::into),
        }
    }

    /// Moves the cursor up to the previous line, attempting to preserve relative offset
    pub fn cursor_up<W: Write>(&mut self, w: &mut W) -> ReplResult<()> {
        // Calculates length of the current line
        let mut len = self.head.len();
        self.cursor_line_start(w)?;
        len -= self.head.len();

        if self.at_start() {
            return Ok(());
        }

        self.cursor_back(w)?;
        self.cursor_line_start(w)?;

        while 0 < len && !self.at_line_end() {
            self.cursor_forward(w)?;
            len -= 1;
        }

        Ok(())
    }

    /// Moves the cursor down to the next line, attempting to preserve relative offset
    pub fn cursor_down<W: Write>(&mut self, w: &mut W) -> ReplResult<()> {
        let mut len = self.head.iter().rev().take_while(|&&c| c != '\n').count();

        self.cursor_line_end(w)?;
        self.cursor_forward(w)?;

        while 0 < len && !self.at_line_end() {
            self.cursor_forward(w)?;
            len -= 1;
        }

        Ok(())
    }

    /// Moves the cursor to the beginning of the current line
    pub fn cursor_line_start<W: Write>(&mut self, w: &mut W) -> ReplResult<()> {
        while !self.at_line_start() {
            self.cursor_back(w)?
        }
        Ok(())
    }

    /// Moves the cursor to the end of the current line
    pub fn cursor_line_end<W: Write>(&mut self, w: &mut W) -> ReplResult<()> {
        while !self.at_line_end() {
            self.cursor_forward(w)?
        }
        Ok(())
    }

    pub fn cursor_start<W: Write>(&mut self, w: &mut W) -> ReplResult<()> {
        while !self.at_start() {
            self.cursor_back(w)?
        }
        Ok(())
    }

    pub fn cursor_end<W: Write>(&mut self, w: &mut W) -> ReplResult<()> {
        while !self.at_end() {
            self.cursor_forward(w)?
        }
        Ok(())
    }
}

impl<'e> IntoIterator for &'e Editor<'_> {
    type Item = &'e char;
    type IntoIter = std::iter::Chain<
        std::collections::vec_deque::Iter<'e, char>,
        std::collections::vec_deque::Iter<'e, char>,
    >;
    fn into_iter(self) -> Self::IntoIter {
        self.head.iter().chain(self.tail.iter())
    }
}

impl Display for Editor<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;
        for c in self.iter() {
            f.write_char(*c)?;
        }
        Ok(())
    }
}
