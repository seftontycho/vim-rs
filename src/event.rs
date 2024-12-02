use std::sync::{mpsc::Sender, Arc, Mutex};

use crossterm::event::KeyCode;

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

impl Operator {
    fn parse(code: KeyCode) -> Option<Operator> {
        todo!()
    }
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

impl Motion {
    fn parse(code: KeyCode) -> Option<Motion> {
        todo!()
    }
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

pub fn event_listener(mode: Arc<Mutex<Mode>>, tx: Sender<Event>) {
    let mut op = None;
    let mut mult = 1;

    while let Ok(ev) = crossterm::event::read() {
        let action = match ev {
            crossterm::event::Event::Key(key_event) => {
                // eprintln!("char {:?}", key_event.code);

                match *mode.lock().unwrap() {
                    Mode::Normal => handle_input_event_normal(key_event.code, &mut op, &mut mult),
                    Mode::Insert => handle_input_event_insert(key_event.code),
                }
            }
            crossterm::event::Event::Resize(cols, rows) => Some(Event::WindowResize(cols, rows)),
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

fn handle_input_event_normal(
    code: KeyCode,
    op: &mut Option<Operator>,
    mult: &mut u8,
) -> Option<Event> {
    if let KeyCode::Char('q') = code {
        return Some(Event::Quit);
    }

    if op.is_none() {
        if let Some(motion) = Motion::parse(code) {
            let m = *mult;
            *mult = 1;

            return Some(Event::Motion {
                op: None,
                mult: m,
                motion: Some(motion),
            });
        }

        if matches!(code, KeyCode::Char('i')) {
            *mult = 1;
            return Some(Event::ChangeMode(Mode::Insert));
        }

        if let Some(o) = Operator::parse(code) {
            op.replace(o);
            return None;
        }

        return None;
    }

    // Cancel current motion
    if matches!(code, KeyCode::Esc) {
        return None;
    }

    if matches!(code, KeyCode::Char('1'..'9')) {
        match code {
            KeyCode::Char(c) => *mult = c as u8 - b'0',
            _ => unreachable!(),
        }

        return None;
    }

    if let Some(motion) = Motion::parse(code) {
        let res = Some(Event::Motion {
            op: op.take(),
            mult: *mult,
            motion: Some(motion),
        });
        *mult = 1;
        return res;
    }

    op.take();
    *mult = 1;
    None
}
