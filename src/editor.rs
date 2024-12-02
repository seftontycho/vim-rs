use crate::buffer::Buffer;
use crate::event::*;

use std::{
    io::{Stdout, Write},
    sync::{Arc, Mutex},
};

use crossterm::{
    cursor, execute, queue,
    style::{PrintStyledContent, Stylize},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};

pub struct Editor {
    stdout: Stdout,
    width: u16,
    height: u16,
    cc: usize,
    cr: usize,
    mode: Arc<Mutex<Mode>>,
    buffer: Buffer,
    paste_buffer: Buffer,
}

impl Editor {
    pub fn new() -> anyhow::Result<Self> {
        Editor::from_string("")
    }

    pub fn from_string(_input: &str) -> anyhow::Result<Self> {
        let stdout = std::io::stdout();
        let (width, height) = terminal::size()?;

        let buffer = Buffer::new();

        Ok(Self {
            stdout,
            width,
            height,
            cc: 0,
            cr: 0,
            mode: Arc::new(Mutex::new(Mode::Normal)),
            buffer,
            paste_buffer: Buffer::new(),
        })
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        self.enter()?;
        self.draw()?;

        let (tx, rx) = std::sync::mpsc::channel();
        let mode = self.mode.clone();
        std::thread::spawn(move || event_listener(mode, tx));

        while let Ok(action) = rx.recv() {
            let done = self.handle_action(action)?;
            self.draw()?;

            if done {
                break;
            }
        }

        self.exit()
    }

    fn enter(&mut self) -> anyhow::Result<()> {
        execute!(self.stdout, EnterAlternateScreen)?;
        terminal::enable_raw_mode()?;
        Ok(())
    }

    fn exit(&mut self) -> anyhow::Result<()> {
        terminal::disable_raw_mode()?;
        execute!(self.stdout, LeaveAlternateScreen)?;
        Ok(())
    }

    fn draw(&mut self) -> anyhow::Result<()> {
        execute!(self.stdout, Clear(ClearType::All))?;

        for (r, row) in self
            .buffer
            .lines
            .iter()
            .enumerate()
            .take(self.height as usize - 2)
        {
            for (c, val) in row.iter().enumerate().take(self.width as usize) {
                queue!(
                    self.stdout,
                    cursor::MoveTo(c as u16, r as u16),
                    PrintStyledContent(val.to_string().magenta())
                )?;
            }
        }

        let mode = self.mode.lock().unwrap().clone();

        queue!(
            self.stdout,
            cursor::MoveTo(0, self.height - 2),
            PrintStyledContent(format!("{:?}", mode).magenta()),
            cursor::MoveTo(self.cc as u16, self.cr as u16)
        )?;

        self.stdout.flush()?;

        Ok(())
    }

    fn handle_action(&mut self, ev: Event) -> anyhow::Result<bool> {
        match ev {
            Event::Quit => return Ok(true),
            Event::ChangeMode(mode) => *self.mode.lock().unwrap() = mode,
            Event::BackSpace => self.handle_backspace()?,
            Event::Enter => self.handle_enter()?,
            Event::Write(char) => self.handle_write(char)?,
            Event::WindowResize(rows, cols) => {
                self.width = cols;
                self.height = rows;
            }
            Event::Motion { op, mult, motion } => self.handle_motion(op, mult, motion),
        }

        Ok(false)
    }

    fn handle_backspace(&mut self) -> anyhow::Result<()> {
        let row = &mut self.buffer.lines[self.cr as usize];

        if (row.is_empty() || self.cc == 0) && self.cr > 0 {
            let row = self.buffer.lines.remove(self.cr as usize);

            self.cr -= 1;
            self.buffer.lines[self.cr as usize].extend(row.into_iter());
            self.cc = self.buffer.lines[self.cr as usize].len() as usize;
            return Ok(());
        }

        if self.cc > 0 {
            row.remove(self.cc as usize - 1);
            self.cc -= 1;
        }

        return Ok(());
    }

    fn handle_write(&mut self, c: char) -> anyhow::Result<()> {
        let row = &mut self.buffer.lines[self.cr as usize];

        if c == '\t' {
            for _ in 0..4 {
                row.insert(self.cc as usize, ' ');
                self.cc += 1;
            }
        }

        if c.is_alphanumeric() || c == ' ' {
            row.insert(self.cc as usize, c);
            self.cc += 1;
        }

        Ok(())
    }

    fn handle_enter(&mut self) -> anyhow::Result<()> {
        let rhs = self.buffer.lines[self.cr as usize].split_off(self.cc as usize);
        self.buffer.lines.insert(self.cr as usize + 1, rhs);
        self.cr += 1;
        self.cc = 0;

        Ok(())
    }

    fn handle_motion(&mut self, op: Option<Operator>, mult: u8, motion: Option<Motion>) {
        let start = (self.cr, self.cc);

        if let Some(motion) = motion {
            for _ in 0..mult as usize {
                match motion {
                    Motion::Up => {
                        self.cr = (self.cr - 1).max(0);
                        let row_len = self.buffer.lines[self.cr].len();
                        self.cc = self.cc.min(row_len);
                    }
                    Motion::Down => {
                        self.cr = (self.cr + 1).min(self.buffer.lines.len());
                        let row_len = self.buffer.lines[self.cr].len();
                        self.cc = self.cc.min(row_len);
                    }
                    Motion::Left => self.cc = (self.cc - 1).max(0),
                    Motion::Right => {
                        let row_len = self.buffer.lines[self.cr].len();
                        self.cc = (self.cc + 1).min(row_len);
                    }
                    Motion::Start => self.cc = 0,
                    Motion::End => {
                        self.cc = self.buffer.lines[self.cr].len();
                    }
                    Motion::Word => {
                        let row = &self.buffer.lines[self.cr];
                        let next_space = row[self.cc..]
                            .iter()
                            .position(|c| *c == ' ')
                            .unwrap_or(row.len());
                        self.cc += next_space;
                    }
                }
            }
        }

        let end = (self.cr, self.cc);

        if let Some(op) = op {
            self.apply_operation(op, start, end);
        }
    }

    fn apply_operation(&self, _op: Operator, _start: (usize, usize), _end: (usize, usize)) {}
}
