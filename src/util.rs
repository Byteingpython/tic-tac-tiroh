use std::sync::Arc;

use crossterm::event::{Event, EventStream};
use futures_lite::StreamExt;
use ratatui::{
    crossterm::event::{KeyCode, KeyEventKind},
    style::Stylize,
    symbols::border,
    text::Line,
    widgets::{Block, Paragraph, Widget},
    DefaultTerminal,
};
use std::str::FromStr;
use tokio::sync::{mpsc, oneshot, Mutex};

use iroh::SecretKey;

use crate::error::Result;

pub fn get_or_create_secret() -> anyhow::Result<SecretKey> {
    if let Ok(secret) = std::env::var("SECRET") {
        let secret = SecretKey::from_str(&secret)?;
        Ok(secret)
    } else {
        let mut rng = rand::rngs::OsRng;
        let secret = SecretKey::generate(&mut rng);
        Ok(secret)
    }
}

#[derive(PartialEq, Clone)]
pub enum Field {
    Empty,
    Server,
    Client,
}

pub enum Role {
    Server,
    Client,
}

pub struct Board {
    playing: bool,
    board: [Field; 9],
}

impl Board {
    pub fn new(playing: bool) -> Self {
        Self {
            playing,
            board: [const { Field::Empty }; 9],
        }
    }
}

impl Widget for &Board {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let mut title = match self.playing {
            true => "Your turn",
            false => "Others turn",
        };

        // TODO: There has to be a better way
        if self.is_win(Field::Server) {
            title = "X wins!";
        } else if self.is_win(Field::Client) {
            title = "O wins!";
        }

        let title = Line::from(title);

        let instructions = Line::from(vec![" <Q>".into(), " Quit ".bold()]);

        let block = Block::bordered()
            .border_set(border::THICK)
            .title(title.centered())
            .title_bottom(instructions);
        let mut fill: [String; 9] = [const { String::new() }; 9];
        for (i, field) in self.board.iter().enumerate() {
            fill[i] = match field {
                Field::Server => "X".to_string(),
                Field::Client => "O".to_string(),
                Field::Empty => match self.playing {
                    true => (i + 1).to_string(),
                    false => " ".to_string(),
                },
            };
        }
        let formatted = format!(
            "     │     │     
  {}  │  {}  │  {}  
     │     │     
─────┼─────┼─────
     │     │     
  {}  │  {}  │  {}  
     │     │     
─────┼─────┼─────
     │     │     
  {}  │  {}  │  {}  
     │     │     
        ",
            fill[0], fill[1], fill[2], fill[3], fill[4], fill[5], fill[6], fill[7], fill[8]
        );
        Paragraph::new(formatted)
            .centered()
            .block(block)
            .render(area, buf);
    }
}

impl Board {
    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn place(&mut self, index: usize, field_type: Field) -> std::result::Result<(), ()> {
        if index > 8 {
            return Err(());
        }
        let field = &self.board[index];
        if *field != Field::Empty {
            return Err(());
        }
        self.board[index] = field_type;
        self.playing = !self.playing;
        Ok(())
    }

    pub fn is_win(&self, field_type: Field) -> bool {
        // Check all rows
        'outer: for i in 0..3 {
            for j in 0..3 {
                let index = i * 3 + j;
                if self.board[index] != field_type {
                    continue 'outer;
                }
            }
            return true;
        }
        // Check all columns
        'outer: for i in 0..3 {
            for j in 0..3 {
                let index = j * 3 + i;
                if self.board[index] != field_type {
                    continue 'outer;
                }
            }
            return true;
        }
        // Check diagonally right
        for i in 0..3 {
            let index = i * 4;
            if self.board[index] != field_type {
                break;
            }
            if i == 2 {
                return true;
            }
        }
        // Check diagonally left
        for i in 0..3 {
            let index = i * 3 + 3 - i;
            if self.board[index] != field_type {
                break;
            }
            if i == 2 {
                return true;
            }
        }
        false
    }
}
