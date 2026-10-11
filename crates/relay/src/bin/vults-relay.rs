//! `vults-relay`: the relay server. It speaks plain HTTP; put it behind TLS (a reverse proxy, or a
//! tailnet) before a phone uses it from outside. `VULTS_RELAY_ADDR` says where to listen
//! (`0.0.0.0:8787` by default).

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let addr = std::env::var("VULTS_RELAY_ADDR").unwrap_or_else(|_| "0.0.0.0:8787".into());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    eprintln!("vults-relay listening on {}", listener.local_addr()?);
    // As a container's first process, a SIGTERM nobody handles is ignored: stop on it.
    tokio::select! {
        served = vults_relay::server::serve(listener, vults_relay::server::Limits::default()) => served,
        () = stopped() => Ok(()),
    }
}

async fn stopped() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        if let Ok(mut term) = signal(SignalKind::terminate()) {
            tokio::select! {
                _ = term.recv() => {}
                _ = tokio::signal::ctrl_c() => {}
            }
            return;
        }
    }
    let _ = tokio::signal::ctrl_c().await;
}
