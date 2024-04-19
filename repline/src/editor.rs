//! The [Editor]

use crossterm::{cursor::*, execute, queue, style::*, terminal::*};
use std::{collections::VecDeque, fmt::Display, io::Write};

use super::error::{Error, ReplResult};

fn is_newline(c: &char) -> bool {
    *c == '\n'
}

fn write_chars<'a, W: Write>(
    c: impl IntoIterator<Item = &'a char>,
    w: &mut W,
) -> std::io::Result<()> {
    for c in c {
        write!(w, "{c}")?;
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct Editor<'a> {
    head: VecDeque<char>,
    tail: VecDeque<char>,

    pub color: &'a str,
    begin: &'a str,
    again: &'a str,
}

impl<'a> Editor<'a> {
    pub fn new(color: &'a str, begin: &'a str, again: &'a str) -> Self {
        Self { head: Default::default(), tail: Default::default(), color, begin, again }
    }
    pub fn iter(&self) -> impl Iterator<Item = &char> {
        let Self { head, tail, .. } = self;
        head.iter().chain(tail.iter())
    }
    pub fn undraw<W: Write>(&self, w: &mut W) -> ReplResult<()> {
        let Self { head, .. } = self;
        match head.iter().copied().filter(is_newline).count() {
            0 => write!(w, "\x1b[0G"),
            lines => write!(w, "\x1b[{}F", lines),
        }?;
        queue!(w, Clear(ClearType::FromCursorDown))?;
        // write!(w, "\x1b[0J")?;
        Ok(())
    }
    pub fn redraw<W: Write>(&self, w: &mut W) -> ReplResult<()> {
        let Self { head, tail, color, begin, again } = self;
        write!(w, "{color}{begin}\x1b[0m ")?;
        // draw head
        for c in head {
            match c {
                '\n' => write!(w, "\r\n{color}{again}\x1b[0m "),
                _ => w.write_all({ *c as u32 }.to_le_bytes().as_slice()),
            }?
        }
        // save cursor
        execute!(w, SavePosition)?;
        // draw tail
        for c in tail {
            match c {
                '\n' => write!(w, "\r\n{color}{again}\x1b[0m "),
                _ => write!(w, "{c}"),
            }?
        }
        // restore cursor
        execute!(w, RestorePosition)?;
        Ok(())
    }
    pub fn prompt<W: Write>(&self, w: &mut W) -> ReplResult<()> {
        let Self { head, color, begin, again, .. } = self;
        queue!(
            w,
            MoveToColumn(0),
            Print(color),
            Print(if head.is_empty() { begin } else { again }),
            ResetColor,
            Print(' '),
        )?;
        Ok(())
    }
    pub fn print_head<W: Write>(&self, w: &mut W) -> ReplResult<()> {
        self.prompt(w)?;
        write_chars(
            self.head.iter().skip(
                self.head
                    .iter()
                    .rposition(is_newline)
                    .unwrap_or(self.head.len())
                    + 1,
            ),
            w,
        )?;
        Ok(())
    }
    pub fn print_tail<W: Write>(&self, w: &mut W) -> ReplResult<()> {
        let Self { tail, .. } = self;
        queue!(w, SavePosition, Clear(ClearType::UntilNewLine))?;
        write_chars(tail.iter().take_while(|&c| !is_newline(c)), w)?;
        queue!(w, RestorePosition)?;
        Ok(())
    }
    pub fn push<W: Write>(&mut self, c: char, w: &mut W) -> ReplResult<()> {
        // Tail optimization: if the tail is empty,
        //we don't have to undraw and redraw on newline
        if self.tail.is_empty() {
            self.head.push_back(c);
            match c {
                '\n' => {
                    write!(w, "\r\n")?;
                    self.print_head(w)?;
                }
                c => {
                    queue!(w, Print(c))?;
                }
            };
            return Ok(());
        }

        if '\n' == c {
            self.undraw(w)?;
        }
        self.head.push_back(c);
        match c {
            '\n' => self.redraw(w)?,
            _ => {
                write!(w, "{c}")?;
                self.print_tail(w)?;
            }
        }
        Ok(())
    }
    pub fn pop<W: Write>(&mut self, w: &mut W) -> ReplResult<Option<char>> {
        if let Some('\n') = self.head.back() {
            self.undraw(w)?;
        }
        let c = self.head.pop_back();
        // if the character was a newline, we need to go back a line
        match c {
            Some('\n') => self.redraw(w)?,
            Some(_) => {
                // go back a char
                queue!(w, MoveLeft(1), Print(' '), MoveLeft(1))?;
                self.print_tail(w)?;
            }
            None => {}
        }
        Ok(c)
    }

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

    pub fn restore(&mut self, s: &str) {
        self.clear();
        self.head.extend(s.chars())
    }
    pub fn clear(&mut self) {
        self.head.clear();
        self.tail.clear();
    }
    pub fn delete<W: Write>(&mut self, w: &mut W) -> ReplResult<char> {
        match self.tail.front() {
            Some('\n') => {
                self.undraw(w)?;
                let out = self.tail.pop_front();
                self.redraw(w)?;
                out
            }
            _ => {
                let out = self.tail.pop_front();
                self.print_tail(w)?;
                out
            }
        }
        .ok_or(Error::EndOfInput)
    }
    pub fn erase_word<W: Write>(&mut self, w: &mut W) -> ReplResult<()> {
        while self.pop(w)?.filter(|c| !c.is_whitespace()).is_some() {}
        Ok(())
    }
    pub fn len(&self) -> usize {
        self.head.len() + self.tail.len()
    }
    pub fn is_empty(&self) -> bool {
        self.head.is_empty() && self.tail.is_empty()
    }
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
    pub fn cursor_back<W: Write>(&mut self, steps: usize, w: &mut W) -> ReplResult<()> {
        for _ in 0..steps {
            if let Some('\n') = self.head.back() {
                self.undraw(w)?;
            }
            let Some(c) = self.head.pop_back() else {
                return Ok(());
            };
            self.tail.push_front(c);
            match c {
                '\n' => self.redraw(w)?,
                _ => queue!(w, MoveLeft(1))?,
            }
        }
        Ok(())
    }
    /// Moves the cursor forward `steps` steps
    pub fn cursor_forward<W: Write>(&mut self, steps: usize, w: &mut W) -> ReplResult<()> {
        for _ in 0..steps {
            if let Some('\n') = self.tail.front() {
                self.undraw(w)?
            }
            let Some(c) = self.tail.pop_front() else {
                return Ok(());
            };
            self.head.push_back(c);
            match c {
                '\n' => self.redraw(w)?,
                _ => queue!(w, MoveRight(1))?,
            }
        }
        Ok(())
    }
    /// Goes to the beginning of the current line
    pub fn home<W: Write>(&mut self, w: &mut W) -> ReplResult<()> {
        loop {
            match self.head.back() {
                Some('\n') | None => break Ok(()),
                Some(_) => self.cursor_back(1, w)?,
            }
        }
    }
    /// Goes to the end of the current line
    pub fn end<W: Write>(&mut self, w: &mut W) -> ReplResult<()> {
        loop {
            match self.tail.front() {
                Some('\n') | None => break Ok(()),
                Some(_) => self.cursor_forward(1, w)?,
            }
        }
    }
}

impl<'a, 'e> IntoIterator for &'e Editor<'a> {
    type Item = &'e char;
    type IntoIter = std::iter::Chain<
        std::collections::vec_deque::Iter<'e, char>,
        std::collections::vec_deque::Iter<'e, char>,
    >;
    fn into_iter(self) -> Self::IntoIter {
        self.head.iter().chain(self.tail.iter())
    }
}
impl<'a> Display for Editor<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;
        for c in self.iter() {
            f.write_char(*c)?;
        }
        Ok(())
    }
}
