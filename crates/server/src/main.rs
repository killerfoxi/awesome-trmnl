use std::{
    net::{Ipv6Addr, SocketAddr},
    path::PathBuf,
    sync::Arc,
};

use clap::Parser;
use eyre::eyre;
use log::info;
use tokio::net::TcpListener;

mod device;
mod error;
mod generator;
mod pages;
mod plugins;
mod resource;
mod serve;
mod storage;

#[derive(Debug, Parser)]
struct Args {
    #[arg(short, long, default_value_t = 8223)]
    port: u16,

    #[arg(short, long)]
    devices_file: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    colog::init();

    let args = Args::parse();
    resource::init_self(args.port);

    let state = serve::ServerState {
        renderer: Arc::new(blender::Instance::new().await.unwrap()),
        storage: Arc::new(
            storage::Storage::load(args.devices_file)
                .await
                .map_err(|e| eyre!("While trying to load local device file: {e}"))?,
        ),
    };

    let listener =
        TcpListener::bind(SocketAddr::new(Ipv6Addr::UNSPECIFIED.into(), args.port)).await?;
    info!(
        "Successfully started listening on {}",
        listener.local_addr()?
    );
    serve::serve(listener, state).await?;
    Ok(())
}
