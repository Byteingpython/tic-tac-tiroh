use std::{io, sync::Arc};

use iroh::{
    endpoint::Connection, protocol::ProtocolHandler, Endpoint
};
use ratatui::{
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph},
    DefaultTerminal, Frame,
};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, sync::{mpsc, oneshot, Mutex}};

use crate::{error::Result, util::{input_loop, read_number, read_q, Board, Field}};

pub struct Server {
    connection: Connection,
    board: Arc<Mutex<Board>>,
}
impl Server {
    pub async fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        let terminal = Arc::new(Mutex::new(terminal));
        let (tx, mut rx) = mpsc::channel(32);
        let (end_tx, end_rx) = oneshot::channel();
        let (mut send, mut recv) = self.connection.accept_bi().await?;
        recv.read_u8().await?;
        {
            let board = self.board.lock().await;
            terminal.lock().await.draw(|frame| frame.render_widget(&*board, frame.area())).unwrap();
        }
        let input_handle = tokio::spawn(input_loop(self.board.clone(), terminal.clone(), tx, end_tx, Field::Server));
        tokio::select! {
            _ = async {
                loop {
                    let index = rx.recv().await.unwrap();
                    send.write_u8(index.try_into().unwrap()).await?;
                    {
                        let board = self.board.lock().await;
                        if board.is_win(Field::Server) {
                            break;
                        }
                    }
                    let index = recv.read_u8().await?; 
                    {
                        let mut board = self.board.lock().await;
                        let _ = board.place(index as usize, Field::Client);
                        if !board.is_playing() {
                            break;
                        }
                        terminal.lock().await.draw(|frame| frame.render_widget(&*board, frame.area())).unwrap();
                        if board.is_win(Field::Server) {
                            break;
                        }
                    }
                }
                Ok::<_, io::Error>(())
            } => {}
            _ = end_rx => {}
        }
        let _ = send.finish();
        self.connection.closed().await;
        input_handle.await??;
        Ok(())
    }


    pub fn new(connection: Connection) -> Self {
        Self {
            connection,
            board: Arc::new(Mutex::new(Board::new(true))),
        }
    }
}
