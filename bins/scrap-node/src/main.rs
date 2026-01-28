use clap::Parser;
use scrap_linux_udp::{load_node_config, run_node, NodeConfig};

#[derive(Parser, Debug)]
#[command(name = "scrap-node", about = "SCRAP edge node (Linux UDP shim)")]
struct Args {
    #[arg(long)]
    config: Option<String>,

    #[arg(long)]
    node_id: Option<String>,

    #[arg(long, default_value = "0.0.0.0")]
    bind: String,

    #[arg(long, default_value_t = 7227)]
    port: u16,

    #[arg(long, default_value = "inventory/routes.json")]
    routes: String,

    #[arg(long, default_value = "demo/runtime/replay_cache.json")]
    replay_cache: String,

    #[arg(long, default_value = "demo/config/revoked.json")]
    revoked: String,

    #[arg(long)]
    commander_pubkey: Option<String>,

    #[arg(long, action = clap::ArgAction::SetTrue)]
    allow_mock_signatures: bool,
}

fn main() {
    let args = Args::parse();
    let config = if let Some(path) = args.config.as_deref() {
        match load_node_config(path) {
            Ok(cfg) => cfg,
            Err(err) => {
                eprintln!("scrap-node config error: {err}");
                std::process::exit(1);
            }
        }
    } else {
        let node_id = match args.node_id {
            Some(value) => value,
            None => {
                eprintln!("scrap-node requires --node-id unless --config is used");
                std::process::exit(1);
            }
        };
        NodeConfig {
            node_id,
            bind: args.bind,
            port: args.port,
            routes_path: args.routes,
            commander_pubkey: args.commander_pubkey,
            replay_cache_path: args.replay_cache,
            revoked_path: args.revoked,
            allow_mock_signatures: args.allow_mock_signatures,
        }
    };

    if let Err(err) = run_node(config) {
        eprintln!("scrap-node failed: {err}");
        std::process::exit(1);
    }
}
