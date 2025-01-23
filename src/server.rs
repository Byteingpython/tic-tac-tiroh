use std::io;

use clap::Parser;
use futures_lite::future::Boxed;
use iroh::{
    discovery::{dns::DnsDiscovery, pkarr::PkarrPublisher, ConcurrentDiscovery},
    endpoint::{self, Connecting, Incoming, VarInt},
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
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::util::{read_number, read_q, Board, Field};

pub struct Server {
    endpoint: Endpoint,
    board: Board,
}
impl Server {
    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        terminal.draw(|frame| self.draw_connect_sceen(frame))?;
        let connection = self.endpoint.accept().await.unwrap().await?;
        let (mut send, mut recv) = connection.accept_bi().await?;
        recv.read_u8().await;
        terminal.clear()?;
        loop {
            terminal.draw(|frame| frame.render_widget(&self.board, frame.area()))?;
            while self.board.is_playing() {
                if let Some(number) = read_number()? {
                    if number < 1 {
                        continue;
                    }
                    let _ = self.board.place(number - 1, Field::Server);
                    if !self.board.is_playing() {
                        send.write_u8((number - 1) as u8).await.unwrap();
                    }
                }
            }
            if self.board.is_win(Field::Server) {
                terminal.draw(|frame| frame.render_widget(&self.board, frame.area()))?;
                break;
            }
            terminal.draw(|frame| frame.render_widget(&self.board, frame.area()))?;
            let number = recv.read_u8().await.unwrap() as usize;
            let _ = self.board.place(number, Field::Client);
            if !self.board.is_playing() {
                // TODO: Error message
                break;
            }
            if self.board.is_win(Field::Client) {
                terminal.draw(|frame| frame.render_widget(&self.board, frame.area()))?;
                break;
            }
        }
        send.finish();
        connection.closed().await;
        read_q()?;
        Ok(())
    }


    fn draw_connect_sceen(&self, frame: &mut Frame) {
        let title = Line::from(" Waiting for connection... ");
        let block = Block::bordered().title(title).border_set(border::THICK);
        let node_id = Text::from(vec![Line::from(vec![
            "Give your peer this id: ".into(),
            self.endpoint.node_id().to_string().into(),
        ])]);

        frame.render_widget(
            Paragraph::new(node_id).centered().block(block),
            frame.area(),
        );
    }

    pub fn new(endpoint: Endpoint) -> Self {
        Self {
            endpoint,
            board: Board::new(true),
        }
    }
}
