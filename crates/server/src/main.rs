#![warn(tail_expr_drop_order)]
#![warn(clippy::pedantic)]
#![allow(clippy::too_many_lines)]

use std::{
    net::{Ipv6Addr, SocketAddr},
    path::PathBuf,
    sync::Arc,
};

use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;
use eyre::eyre;
use log::info;

mod device;
mod error;
mod generator;
mod pages;
mod plugins;
mod resource;
mod serve;
mod storage;

#[derive(Parser)]
#[command(rename_all = "snake_case")]
struct Args {
    #[arg(short, long, default_value_t = 8223, help = "Port to listen on.")]
    port: u16,

    #[arg(
        short,
        long,
        help = "Path to devices.toml storing device configuration."
    )]
    devices_file: Option<PathBuf>,

    #[arg(
        long,
        default_value_t = false,
        help = "Shows detailed request processing (debugging)."
    )]
    show_request_details: bool,

    #[arg(
        long,
        help = "Override default chromium based browser profile directory."
    )]
    user_dir: Option<PathBuf>,

    #[command(flatten)]
    tls: TlsArgs,
}

#[derive(clap::Args)]
#[command(rename_all = "snake_case")]
struct TlsArgs {
    #[arg(long, default_value_t = false, help = "Disables TLS.")]
    nouse_tls: bool,

    #[arg(long, required_unless_present = "nouse_tls")]
    cert_file: Option<PathBuf>,
    #[arg(long, required_unless_present = "nouse_tls")]
    key_file: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    pretty_env_logger::formatted_builder()
        .filter_module("atrmnl_server", log::LevelFilter::Info)
        .filter_module("chromiumoxide", log::LevelFilter::Off)
        .parse_env("RUST_LOG")
        .init();

    let args = Args::parse();
    resource::init_self(args.port, !args.tls.nouse_tls);

    let tls = if args.tls.nouse_tls {
        None
    } else {
        Some(
            args.tls
                .cert_file
                .zip(args.tls.key_file)
                .map(|(cert, key)| RustlsConfig::from_pem_file(cert, key))
                .expect("Cert and key provided")
                .await?,
        )
    };

    let state = serve::ServerState {
        renderer: Arc::new(blender::Instance::new(args.user_dir).await.unwrap()),
        storage: Arc::new(
            storage::Storage::load(args.devices_file)
                .await
                .map_err(|e| eyre!("While trying to load local device file: {e}"))?,
        ),
    };

    let addr = SocketAddr::new(Ipv6Addr::UNSPECIFIED.into(), args.port);
    info!("Starting listening on {}", addr);
    serve::serve(addr, tls, state, args.show_request_details).await?;
    Ok(())
}
