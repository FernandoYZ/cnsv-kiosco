mod app;
mod assets;
mod auth;
mod backup;
mod cierre;
mod cobranza;
mod config;
mod consumos;
mod dashboard;
mod db;
mod error;
mod estudiantes;
mod fechas;
mod forms;
mod pagos;
mod productos;
mod rutas;
mod state;
mod templates;
#[cfg(test)]
mod test_support;

use std::{net::SocketAddr, process::ExitCode};

use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use crate::{auth::Auth, config::Config, error::StartupError, state::AppState};

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info")),
        )
        .init();

    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            tracing::error!(error = %err, "server stopped with an error");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), StartupError> {
    let config = Config::load()?;
    let db = db::open(&config.database.path).await?;
    backup::programar(db.clone(), config.backup.clone());
    let state = AppState {
        db: db.clone(),
        envios_pago: Default::default(),
        auth: Auth::new(config.auth.cookie_segura),
    };

    let listener = TcpListener::bind(config.server.address).await?;
    tracing::info!(address = %config.server.address, "server listening");

    axum::serve(
        listener,
        // Peer address for the login rate limit when there is no proxy.
        app::router(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    db.close().await?;
    tracing::info!("server stopped");
    Ok(())
}

/// Resolves on Ctrl+C or, on Unix, on SIGTERM (what systemd sends on stop).
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %err, "cannot listen for Ctrl+C");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(err) => {
                tracing::error!(error = %err, "cannot listen for SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
