use std::{io, sync::{Arc}};

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use futures_lite::StreamExt;
use iroh::endpoint::{Connection, VarInt};
use ratatui::DefaultTerminal;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, sync::{mpsc::{self, Sender, UnboundedSender}, oneshot, Mutex}};

use crate::{error::Result, util::{input_loop, read_number, read_q, Board, Field}};

pub struct Client {
    connection: Connection,
    board: Arc<Mutex<Board>>,
    end: bool,
}
impl Client {
    pub async fn run(&mut self, terminal: DefaultTerminal) -> Result<()> {
        let terminal = Arc::new(Mutex::new(terminal));
        let (tx, mut rx) = mpsc::channel(32);
        let (end_tx, end_rx) = oneshot::channel();
        let (mut send, mut recv) = self.connection.open_bi().await?;
        send.write_u8(0).await?;
        {
            let board = self.board.lock().await;
            terminal.lock().await.draw(|frame| frame.render_widget(&*board, frame.area())).unwrap();
        }
        let input_handle = tokio::spawn(input_loop(self.board.clone(), terminal.clone(), tx, end_tx, Field::Client));
        tokio::select! {
            _ = async {
                loop {
                    let index = recv.read_u8().await?; 
                    {
                        let mut board = self.board.lock().await;
                        let _ = board.place(index as usize, Field::Server);
                        if !board.is_playing() {
                            break;
                        }
                        terminal.lock().await.draw(|frame| frame.render_widget(&*board, frame.area())).unwrap();
                        if board.is_win(Field::Server) {
                            break;
                        }
                    }
                    let index = rx.recv().await.unwrap();
                    send.write_u8(index.try_into().unwrap()).await?;
                    {
                        let board = self.board.lock().await;
                        if board.is_win(Field::Client) {
                            break;
                        }
                    }
                }
                Ok::<_, io::Error>(())
            } => {}
            _ = end_rx => {}
        }
        let _ = send.finish();
        self.connection.close(VarInt::from_u32(0), &[]);
        input_handle.await?;
        Ok(())
    }



    pub fn new(connection: Connection) -> Self {
        Self {
            connection,
            board: Arc::new(Mutex::new(Board::new(false))),
            end: false,
        }
    }
}
