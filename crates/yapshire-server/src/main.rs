use std::path::PathBuf;
use yapshire_server::{Config, Server};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("yapshire-server: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mut config_path = None;
    let mut bind = None;
    let mut maps = None;
    let mut check = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!(
                    "Yapshire server {}\n\nUsage: yapshire-server [--config server.json] [--bind 0.0.0.0:4761] [--maps ./maps] [--check]\n       yapshire-server --init ./my-town\n\nNo configuration is required to run the bundled town. --init writes editable Tiled maps and server.json without overwriting files.\nSet YAPSHIRE_SERVER_PASSWORD for an optional shared server password (8-128 bytes). Use a TLS reverse proxy for public wss:// access.\n--check validates configuration and maps without opening a port.",
                    env!("CARGO_PKG_VERSION")
                );
                return Ok(());
            }
            "--version" | "-V" => {
                println!("yapshire-server {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--init" => {
                let folder = PathBuf::from(args.next().ok_or("--init needs a folder")?);
                if args.next().is_some() {
                    return Err("--init must be used by itself".into());
                }
                Config::initialize(&folder)?;
                println!(
                    "Created {}. Run yapshire-server --config {}",
                    folder.display(),
                    folder.join("server.json").display()
                );
                return Ok(());
            }
            "--config" => {
                config_path = Some(PathBuf::from(args.next().ok_or("--config needs a file")?))
            }
            "--bind" => bind = Some(args.next().ok_or("--bind needs an address")?.parse()?),
            "--maps" => maps = Some(PathBuf::from(args.next().ok_or("--maps needs a folder")?)),
            "--check" => check = true,
            _ => return Err(format!("Unknown argument {arg}; use --help").into()),
        }
    }
    if config_path.is_none() && std::path::Path::new("server.json").exists() {
        config_path = Some("server.json".into());
    }
    let mut config = config_path
        .as_ref()
        .map_or_else(|| Ok(Config::default()), |path| Config::load(path))?;
    if let Some(bind) = bind {
        config.bind = bind;
    }
    if maps.is_some() {
        config.maps_dir = maps;
    }
    let world = config.world()?;
    let password = std::env::var("YAPSHIRE_SERVER_PASSWORD").unwrap_or_default();
    let server = Server::new(config.clone(), world, &password)?;
    if check {
        println!(
            "Valid configuration and shared Tiled maps. World: {}",
            server.revision()
        );
        return Ok(());
    }
    let listener = std::net::TcpListener::bind(config.bind)?;
    println!(
        "Yapshire server {} listening on {}\nTown: {} / {}\nWorld: {}\nAccess: {}",
        env!("CARGO_PKG_VERSION"),
        listener.local_addr()?,
        config.name,
        config.room_code,
        server.revision(),
        if password.is_empty() {
            "public"
        } else {
            "password required"
        }
    );
    server.run(listener, shutdown()).await?;
    println!("Server stopped.");
    Ok(())
}

async fn shutdown() {
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("SIGTERM handler");
        tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = term.recv() => {} }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
