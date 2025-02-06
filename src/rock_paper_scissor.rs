use std::sync::Arc;

use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
    Key
};
use crossterm::{event::{Event, EventStream, KeyCode, KeyEventKind}};
use futures_lite::StreamExt;
use iroh::endpoint::Connection;
use ratatui::{
    style::Stylize,
    symbols::border,
    text::Line,
    widgets::{Block, Paragraph, Widget}, DefaultTerminal,
};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, sync::{mpsc, Mutex}};

use crate::{error::{Error, Result}, util::Role};

#[derive(Debug, Clone, PartialEq)]
pub enum Guess {
    Rock,
    Paper,
    Scissors,
}

impl TryFrom<u32> for Guess {
    type Error = Error;
    fn try_from(value: u32) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(Guess::Rock),
            1 => Ok(Guess::Paper),
            2 => Ok(Guess::Scissors),
            _ => Err(Error::ConversionError),
        }
    }
}

impl From<&Guess> for u8 {
    fn from(value: &Guess) -> Self {
        match value {
            Guess::Rock => 0,
            Guess::Paper => 1,
            Guess::Scissors => 2,
        }
    }
}

impl Guess {
    pub fn encrypt(&self) -> Result<(Vec<u8>, Key, Nonce)> {
        let key = ChaCha20Poly1305::generate_key(&mut OsRng);
        let cipher = ChaCha20Poly1305::new(&key);
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        let data = u8::from(self);
        let encrypted = cipher.encrypt(&nonce, [data].as_slice())?;
        Ok((encrypted, key, nonce))
    }
    
    pub fn decrypt(encrypted: &[u8], key: &[u8], nonce: &[u8]) -> Result<Self> {
        if key.len() != 32 || nonce.len() != 12 {
            return Err(Error::SizeError);
        }
        let key = Key::from_slice(key);
        let nonce = Nonce::from_slice(nonce);
        let cipher = ChaCha20Poly1305::new(&key);
        let decrypted = cipher.decrypt(&nonce, encrypted)?;
        if decrypted.len() != 1 {
            return Err(Error::SizeError);
        }
        let data = decrypted.get(0).unwrap();
        Ok(Self::try_from(*data as u32)?)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum State {
    Waiting,
    Guess(Guess),
    Result(bool),
}

#[derive(Clone)]
pub struct RockPaperScissors {
    role: Arc<Role>,
    terminal: Arc<Mutex<DefaultTerminal>>,
    state: Arc<Mutex<State>>,
}

impl RockPaperScissors {
    pub fn new(role: Role, terminal: DefaultTerminal) -> Self {
        RockPaperScissors {
            role: role.into(),
            terminal: Mutex::new(terminal).into(),
            state: Mutex::new(State::Waiting).into(),
        }
    }

    async fn input_thread(&self, channel: mpsc::Sender<Guess>) -> Result<()> {
        let mut stream = EventStream::new();
        while let Some(event) = stream.next().await {
            let event = event?;
            match event {
                Event::Resize(_, _) => {
                    self.render().await?;
                },
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    if let KeyCode::Char(c) = key_event.code {
                        if c.is_digit(10) {
                            let index = c.to_digit(10).unwrap() - 1;
                            if *self.state.lock().await != State::Waiting {
                                continue;
                            }
                            let guess: Guess = index.try_into()?;
                            channel.send(guess.clone()).await?;
                            *self.state.lock().await = State::Guess(guess);
                            self.render().await?;
                        } else if c == 'q' {
                            return Ok(());
                        }
                    }
                }
                _ => {},
            }
        }
        Err(crate::error::Error::InputAbort)
    }

    async fn connection_thread(&self, connection: Connection, mut input_receiver: mpsc::Receiver<Guess>) -> Result<()> {
        let (mut send, mut recv) = match *self.role {
            Role::Client => {
                let (mut send, recv) = connection.open_bi().await?;
                send.write_u8(0).await?;
                (send, recv)
            }
            Role::Server => {
                let (send, mut recv) = connection.accept_bi().await?;
                let _ = recv.read_u8().await?;
                (send, recv)
            }
        };
        let guess: Guess = input_receiver.recv().await.unwrap();
        let other_guess = match *self.role {
            Role::Client => {
                let (encrypted, key, nonce) = guess.encrypt()?;
                send.write(encrypted.as_slice()).await?;
                let other_guess = Guess::try_from(recv.read_u8().await? as u32)?;
                send.write(key.as_slice()).await?;
                send.write(nonce.as_slice()).await?;
                other_guess
            },
            Role::Server => {
                // TODO: this is probably to large
                let mut encrypted = Vec::new();
                // TODO: Remove unwrap 
                recv.read(&mut encrypted).await?.unwrap();
                send.write_u8((&guess).into()).await?;
                let mut key = Vec::new();
                recv.read(&mut key).await?.unwrap();
                let mut nonce = Vec::new();
                recv.read(&mut nonce).await?.unwrap();
                Guess::decrypt(&encrypted, &key, &nonce)?
            },
        };
        *self.state.lock().await = State::Result(Self::is_win(guess, other_guess));
        self.render().await?;
        Ok(())
    }

    fn is_win(your_guess: Guess, other_guess: Guess) -> bool {
        if your_guess == Guess::Rock && other_guess == Guess::Scissors {
            return true;
        }
        if your_guess == Guess::Paper && other_guess == Guess::Rock {
            return true;
        }
        if your_guess == Guess::Scissors && other_guess == Guess::Paper {
            return true;
        }
        false
    }

    async fn render(&self) -> Result<()> {
        let title = Line::from("Rock, Paper, Scissors");
        let instructions = Line::from(vec![" <Q>".into(), " Quit ".bold()]);

        let block = Block::bordered()
            .border_set(border::THICK)
            .title(title.centered())
            .title_bottom(instructions);
        
        let text = match *self.state.lock().await {
            State::Waiting => "Input your guess: Rock (1), Paper (2) or Scissors (3)?",
            State::Guess(_) => "Waiting for your opponent to respond",
            State::Result(result) => {
                if result{"You won!"} else {"You lost!"}
            }
        }.to_string();

        let paragraph = Paragraph::new(text)
            .centered()
            .block(block);
        
        self.terminal.lock().await.draw(|frame| frame.render_widget(&paragraph, frame.area()))?;
        Ok(())
    }

    async fn run(&self, connection: Connection) -> Result<()> {
        let cloned_self = self.clone();
        // TODO: Too big
        let (input_sender, input_receiver) = mpsc::channel(4);
        let connection_handle = tokio::spawn(async move {
            cloned_self.connection_thread(connection, input_receiver).await
        });
        self.input_thread(input_sender).await?;
        connection_handle.abort();
        Ok(())
    }
}
