use clap::{Parser, ValueEnum};
use client::Client;
use error::Error;
use iroh::{
    discovery::{dns::DnsDiscovery, pkarr::PkarrPublisher, ConcurrentDiscovery},
    Endpoint, NodeId,
};
use server::Server;
use tic_tac_toe::TicTacToe;
use util::{get_or_create_secret, Role};

mod client;
mod error;
mod server;
mod tic_tac_toe;
mod util;

const WEB3_ALPN: &[u8] = b"WEB3_2024";

/**
   _______         ______              _______            __
  /_  __(_)____   /_  __/___ ______   /_  __(_)________  / /_
   / / / / ___/    / / / __ `/ ___/    / / / / ___/ __ \/ __ \
  / / / / /__     / / / /_/ / /__     / / / / /  / /_/ / / / /
 /_/ /_/\___/    /_/  \__,_/\___/    /_/ /_/_/   \____/_/ /_/

 Tic Tac Toe over the Iroh p2p protocol
*/
#[derive(Debug, Parser)]
#[clap(verbatim_doc_comment)]
struct Args {
    /// The game you want to play
    #[arg(value_enum)]
    game: GameMode,
    /// The ID of your peer. Leave blank to generate a new ID
    id: Option<NodeId>,
}

#[derive(ValueEnum, Debug, PartialEq, Clone, Copy, Eq, PartialOrd, Ord)]
#[clap(rename_all = "kebab_case")]
enum GameMode {
    TicTacToe,
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
    let connection = match args.id {
        Some(id) => endpoint.connect(id, WEB3_ALPN).await?,
        None => {
            println!("Give your peer this ID: {}", endpoint.node_id());
            println!("Waiting for connection...");
            endpoint.accept().await.unwrap().await?
        }
    };
    let terminal = ratatui::init();
    let result = match args.id {
        Some(_) => {
            let mut client = Client::new(connection);
            client.run(terminal).await?;
            error::Result::Ok(())
        }
        None => {
            let tic_tac_toe = TicTacToe::new(Role::Server, terminal);
            tic_tac_toe.run(connection).await
        }
    };
    ratatui::restore();
    result?;
    Ok(())
}
