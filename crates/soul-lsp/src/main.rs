mod cli;
mod server;

// ---------------------------------------------------------------------------------------------- //

use cli::Cli;
use server::Server as SoulLspServer;

use clap::Parser;
use tower_lsp_server::LspService;
use tower_lsp_server::Server as TowerLspServer;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let (service, socket) = LspService::new(|client| SoulLspServer::new(client, cli.root));
    TowerLspServer::new(tokio::io::stdin(), tokio::io::stdout(), socket)
        .serve(service)
        .await;
}
