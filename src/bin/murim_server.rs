use muc_engine::server::{MudServer, ServerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    tracing_subscriber::fmt::init();

    // Create server configuration
    let config = ServerConfig::new("0.0.0.0", 9999);

    // Create and run the server
    let server = MudServer::new(config);
    server.run().await?;

    Ok(())
}