use std::sync::Arc;

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use futures_lite::StreamExt;
use iroh::endpoint::Connection;
use ratatui::DefaultTerminal;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{mpsc, Mutex},
};

use crate::{
    error::Result,
    util::{Board, Field, Role},
};

#[derive(Clone)]
pub struct TicTacToe {
    board: Arc<Mutex<Board>>,
    role: Arc<Role>,
    terminal: Arc<Mutex<DefaultTerminal>>,
}

impl TicTacToe {
    pub fn new(role: Role, terminal: DefaultTerminal) -> Self {
        TicTacToe {
            board: Arc::new(Mutex::new(Board::new(match role {
                Role::Server => true,
                Role::Client => false,
            }))),
            role: role.into(),
            terminal: Arc::new(Mutex::new(terminal)),
        }
    }

    async fn connection_thread(
        &self,
        connection: Connection,
        mut input_receiver: mpsc::Receiver<u32>,
    ) -> Result<()> {
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

        loop {
            let playing = self.board.lock().await.is_playing();
            if playing {
                let index = input_receiver.recv().await.unwrap();
                send.write_u8(index.try_into().unwrap()).await?;
                {
                    let board = self.board.lock().await;
                    if board.is_win(Field::Server) {
                        break;
                    }
                }
            } else {
                let index = recv.read_u8().await?;
                {
                    let mut board = self.board.lock().await;
                    let _ = board.place(index as usize, Field::Client);
                    if !board.is_playing() {
                        break;
                    }
                    self.terminal
                        .lock()
                        .await
                        .draw(|frame| frame.render_widget(&*board, frame.area()))
                        .unwrap();
                    if board.is_win(Field::Server) {
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    async fn input_loop(self, channel: mpsc::Sender<u32>) -> Result<()> {
        let field_type = match *self.role {
            Role::Server => Field::Server,
            Role::Client => Field::Client,
        };
        let mut stream = EventStream::new();
        while let Some(event) = stream.next().await {
            let event = event?;
            match event {
                Event::Resize(_, _) => {
                    let board = self.board.lock().await;
                    self.terminal
                        .lock()
                        .await
                        .draw(|frame| frame.render_widget(&*board, frame.area()))?;
                }
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    let mut board = self.board.lock().await;
                    if let KeyCode::Char(c) = key_event.code {
                        if c.is_digit(10) {
                            if !board.is_playing() {
                                continue;
                            }
                            let index = c.to_digit(10).unwrap() - 1;
                            let _ = board.place(index as usize, field_type.clone());
                            if board.is_playing() {
                                continue;
                            }
                            channel.send(index).await?;
                            self.terminal
                                .lock()
                                .await
                                .draw(|frame| frame.render_widget(&*board, frame.area()))?;
                        } else if c == 'q' {
                            return Ok(());
                        }
                    }
                }
                _ => {}
            }
        }
        Err(crate::error::Error::InputAbort)
    }

    pub async fn run(&self, connection: Connection) -> Result<()> {
        {
            let board = self.board.lock().await;
            self.terminal
                .lock()
                .await
                .draw(|frame| frame.render_widget(&*board, frame.area()))?;
        }
        let (input_sender, input_receiver) = mpsc::channel(32);
        let cloned_self = self.clone();
        let connection_handle = tokio::spawn(async move {
            cloned_self
                .connection_thread(connection, input_receiver)
                .await
        });
        self.clone().input_loop(input_sender).await?;
        connection_handle.abort();
        Ok(())
    }
}
