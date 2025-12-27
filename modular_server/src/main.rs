use clap::{Arg, Command};
use modular_server::{ServerConfig, run_server};
use tokio::task;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let matches = Command::new("Modular")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("HTTP/WebSocket server for modular audio synthesis")
        .arg(
            Arg::new("port")
                .long("port")
                .short('p')
                .value_name("PORT")
                .default_value("7812")
                .num_args(1),
        )
        .get_matches();

    let port = matches.get_one::<String>("port").unwrap().parse::<u16>()?;

    let config = ServerConfig {
        port,
        patch_file: None,
    };

    run_server(config).await
}
