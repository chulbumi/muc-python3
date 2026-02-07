use muc_engine::server::{MudServer, ServerConfig};

fn get_port() -> u16 {
    // 1) 커맨드라인: cargo run --bin murim_server -- 9990
    if let Some(arg) = std::env::args().nth(1) {
        if let Ok(p) = arg.parse::<u16>() {
            return p;
        }
    }
    // 2) 환경변수 MUD_PORT 또는 PORT
    if let Ok(s) = std::env::var("MUD_PORT") {
        if let Ok(p) = s.parse::<u16>() {
            return p;
        }
    }
    if let Ok(s) = std::env::var("PORT") {
        if let Ok(p) = s.parse::<u16>() {
            return p;
        }
    }
    // 3) 기본값
    9999
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    tracing_subscriber::fmt::init();

    let port = get_port();
    tracing::info!("MUD server binding to 0.0.0.0:{}", port);

    // Create server configuration
    let config = ServerConfig::new("0.0.0.0", port);

    // Create and run the server
    let server = MudServer::new(config);
    server.run().await?;

    Ok(())
}
