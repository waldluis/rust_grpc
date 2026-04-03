mod backend;

use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::Server;
use tracing::info;

use backend::{
    backend_server::BackendServer,
    BackendService, BackendState,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),).init();

    let address = "[::1]:50051".parse()?;
    info!(%address, "starting gRPC backend");

    let state = Arc::new(Mutex::new(BackendState::new()));
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let service = BackendService::new(Arc::clone(&state), shutdown_tx);

    Server::builder()
        .add_service(BackendServer::new(service))
        .serve_with_shutdown(address, async {
            let _ = shutdown_rx.await;
            info!("shutdown signal received - stopping server");
        })
        .await?;

    info!("server stopped");
    Ok(())
}



