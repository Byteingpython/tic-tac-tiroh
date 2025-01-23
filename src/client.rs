use std::io;

use clap::Parser;
use crossterm::event::EventStream;
use futures_lite::{future::Boxed, StreamExt};
use iroh::{
    discovery::{dns::DnsDiscovery, pkarr::PkarrPublisher, ConcurrentDiscovery},
    endpoint::{self, Connecting, Connection, Incoming, VarInt},
    protocol::{ProtocolHandler, Router},
    Endpoint, NodeId, PublicKey,
};
use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind},
    layout::Layout,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
    DefaultTerminal, Frame,
};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, sync::oneshot};

use crate::util::{read_number, read_q, Board, Field};

pub struct Client {
    connection: Connection,
    board: Board,
    end: bool,
}
impl Client {
    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let (mut send, mut recv) = self.connection.open_bi().await?;
        send.write_u8(0).await;
        loop {
            terminal.draw(|frame| frame.render_widget(&self.board, frame.area()))?;
            let number = recv.read_u8().await.unwrap() as usize;
            let _ = self.board.place(number, Field::Server);
            if !self.board.is_playing() {
                // TODO: Error message
                break;
            }
            if self.board.is_win(Field::Server) {
                terminal.draw(|frame| frame.render_widget(&self.board, frame.area()))?;
                break;
            }
            terminal.draw(|frame| frame.render_widget(&self.board, frame.area()))?;
            while self.board.is_playing() {
                if let Some(number) = read_number()? {
                    if number < 1 {
                        continue;
                    }
                    let _ = self.board.place(number - 1, Field::Client);
                    if !self.board.is_playing() {
                        send.write_u8((number - 1) as u8).await.unwrap();
                    }
                }
            }
            if self.board.is_win(Field::Client) {
                terminal.draw(|frame| frame.render_widget(&self.board, frame.area()))?;
                break;
            }
        }
        send.finish();
        self.connection.close(VarInt::from_u32(0), &[]);
        read_q()?;
        Ok(())
    }


    pub fn new(connection: Connection) -> Self {
        Self {
            connection,
            board: Board::new(false),
            end: false,
        }
    }
}
