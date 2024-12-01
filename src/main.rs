use std::{
    any,
    io::{Stdout, Write},
    sync::{mpsc::Sender, Arc, Mutex},
};

use crossterm::{
    cursor,
    event::{read, Event, KeyCode, KeyEvent},
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
pub enum Action {
    Quit,
    ChangeMode(Mode),
    Write(char),
    Motion {
        op: Option<Operator>,
        mult: u8,
        motion: Option<Motion>,
    },
    WindowResize(u16, u16),
}

pub fn event_listener(mode: Arc<Mutex<Mode>>, tx: Sender<Action>) {
    let mut keys = Vec::new();

    while let Ok(ev) = read() {
        let action = match ev {
            Event::Key(key_event) => match *mode.lock().unwrap() {
                Mode::Normal => handle_normal_event(key_event.code, &mut keys),
                Mode::Insert => handle_insert_event(key_event.code),
            },
            Event::Resize(cols, rows) => Some(Action::WindowResize(cols, rows)),
            _ => None,
        };

        if let Some(action) = action {
            tx.send(action).unwrap();
        }
    }
}

fn handle_insert_event(code: KeyCode) -> Option<Action> {
    match code {
        KeyCode::Esc => Some(Action::ChangeMode(Mode::Normal)),
        KeyCode::Char(c) => Some(Action::Write(c)),
        _ => None,
    }
}

fn handle_normal_event(code: KeyCode, keys: &mut Vec<char>) -> Option<Action> {
    match code {
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Char('i') => Some(Action::ChangeMode(Mode::Insert)),
        KeyCode::Left => Some(Action::Motion {
            op: None,
            mult: 1,
            motion: Some(Motion::Left),
        }),
        KeyCode::Right => Some(Action::Motion {
            op: None,
            mult: 1,
            motion: Some(Motion::Right),
        }),
        KeyCode::Up => Some(Action::Motion {
            op: None,
            mult: 1,
            motion: Some(Motion::Up),
        }),
        KeyCode::Down => Some(Action::Motion {
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
    buffer: String,
}

impl Editor {
    pub fn new() -> anyhow::Result<Self> {
        let stdout = std::io::stdout();
        let (width, height) = terminal::size()?;

        Ok(Self {
            stdout,
            width,
            height,
            cc: 0,
            cr: 0,
            mode: Arc::new(Mutex::new(Mode::Normal)),
            buffer: String::new(),
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
        queue!(self.stdout, cursor::MoveTo(self.cc, self.cr))?;

        for c in self.buffer.chars() {
            if c == '\n' {
                self.cc = 0;
                self.cr += 1;
                continue;
            }

            queue!(self.stdout, PrintStyledContent(c.to_string().magenta()))?;
        }

        self.stdout.flush()?;

        Ok(())
    }

    fn handle_action(&self, action: Action) -> _ {
        todo!()
    }
}

fn main() -> anyhow::Result<()> {
    let mut editor = Editor::new()?;

    editor.run()?;

    Ok(())
}
