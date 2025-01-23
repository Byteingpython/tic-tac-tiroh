use std::io;

use clap::Parser;
use client::Client;
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
use server::Server;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use util::{get_or_create_secret, Board};

mod util;
mod server;
mod client;

const WEB3_ALPN: &[u8] = b"WEB3_2024";

#[derive(Debug, Parser)]
struct Args {
    id: Option<NodeId>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let secret_key = get_or_create_secret()?;
    let discovery = Box::new(ConcurrentDiscovery::from_services(vec![
        Box::new(DnsDiscovery::n0_dns()),
        Box::new(PkarrPublisher::n0_dns(secret_key.clone())),
    ]));

    let endpoint = Endpoint::builder()
        .secret_key(secret_key.clone())
        .alpns(vec![WEB3_ALPN.to_vec()])
        .discovery(discovery)
        .bind()
        .await?;
    let mut terminal = ratatui::init();
    let result = match args.id {
        Some(id) => {
            let connection = endpoint.connect(id, WEB3_ALPN).await?;
            let mut client = Client::new(connection);
            client.run(&mut terminal).await
        }
        None => {
            let mut server = Server::new(endpoint);
            server.run(&mut terminal).await
        }
    };
    ratatui::restore();
    result?;
    Ok(())
}

