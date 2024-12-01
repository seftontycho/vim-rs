use crate::buffer::Buffer;

use std::{
    io::{Stdout, Write},
    sync::{mpsc::Sender, Arc, Mutex},
};

use crossterm::{
    cursor,
    event::{self, read, KeyCode},
    execute, queue,
    style::{PrintStyledContent, Stylize},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Mode {
    Normal,
    Insert,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Operator {
    Delete,
    Yank,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Motion {
    Up,
    Down,
    Left,
    Right,

    Start,
    End,

    Word,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Event {
    Quit,
    ChangeMode(Mode),
    Write(char),
    BackSpace,
    Enter,
    Motion {
        op: Option<Operator>,
        mult: u8,
        motion: Option<Motion>,
    },
    WindowResize(u16, u16),
}

fn event_listener(mode: Arc<Mutex<Mode>>, tx: Sender<Event>) {
    let mut keys = Vec::new();

    while let Ok(ev) = read() {
        let action = match ev {
            event::Event::Key(key_event) => {
                // eprintln!("char {:?}", key_event.code);

                match *mode.lock().unwrap() {
                    Mode::Normal => handle_input_event_normal(key_event.code, &mut keys),
                    Mode::Insert => handle_input_event_insert(key_event.code),
                }
            }
            event::Event::Resize(cols, rows) => Some(Event::WindowResize(cols, rows)),
            _ => None,
        };

        if let Some(action) = action {
            tx.send(action).unwrap();
        }
    }
}

fn handle_input_event_insert(code: KeyCode) -> Option<Event> {
    match code {
        KeyCode::Esc => Some(Event::ChangeMode(Mode::Normal)),
        KeyCode::Char(c) => Some(Event::Write(c)),
        KeyCode::Tab => Some(Event::Write('\t')),
        KeyCode::Backspace => Some(Event::BackSpace),
        KeyCode::Enter => Some(Event::Enter),

        _ => None,
    }
}

fn handle_input_event_normal(code: KeyCode, _keys: &mut Vec<char>) -> Option<Event> {
    match code {
        KeyCode::Char('q') => Some(Event::Quit),
        KeyCode::Char('i') => Some(Event::ChangeMode(Mode::Insert)),
        KeyCode::Left => Some(Event::Motion {
            op: None,
            mult: 1,
            motion: Some(Motion::Left),
        }),
        KeyCode::Right => Some(Event::Motion {
            op: None,
            mult: 1,
            motion: Some(Motion::Right),
        }),
        KeyCode::Up => Some(Event::Motion {
            op: None,
            mult: 1,
            motion: Some(Motion::Up),
        }),
        KeyCode::Down => Some(Event::Motion {
            op: None,
            mult: 1,
            motion: Some(Motion::Down),
        }),
        _ => None,
    }
}

pub struct Editor {
    stdout: Stdout,
    width: u16,
    height: u16,
    cc: u16,
    cr: u16,
    mode: Arc<Mutex<Mode>>,
    buffer: Buffer,
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
            cursor::MoveTo(self.cc, self.cr)
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
            _ => {}
        }

        Ok(false)
    }

    fn handle_backspace(&mut self) -> anyhow::Result<()> {
        let row = &mut self.buffer.lines[self.cr as usize];

        if (row.is_empty() || self.cc == 0) && self.cr > 0 {
            let row = self.buffer.lines.remove(self.cr as usize);

            self.cr -= 1;
            self.buffer.lines[self.cr as usize].extend(row.into_iter());
            self.cc = self.buffer.lines[self.cr as usize].len() as u16;
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
}
