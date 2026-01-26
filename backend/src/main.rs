mod camera;
mod config;

use std::{convert::Infallible, net::SocketAddr, sync::Arc};

use anyhow::Context;
use axum::{
    body::Body,
    extract::State,
    http::{header, Method, StatusCode},
    response::{AppendHeaders, IntoResponse, Response},
    routing::get,
    Json, Router,
};
use bytes::{Bytes, BytesMut};
#[cfg(target_os = "linux")]
use camera::{V4l2Camera, RpiCamCamera};
use camera::{Camera, MockCamera};
use config::Config;
use tokio::{net::TcpListener, signal, time::interval};
use tokio_util::sync::CancellationToken;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Clone)]
struct AppState {
    camera: Arc<dyn Camera>,
    config: Config,
    shutdown: CancellationToken,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing()?;

    let config = Config::from_env()?;
    tracing::info!(?config, "Loaded configuration");

    let camera = build_camera(&config);
    let shutdown = CancellationToken::new();

    let state = AppState { camera, config, shutdown: shutdown.clone() };
    let addr: SocketAddr = state.config.listen_socket_addr();

    let app = Router::new()
        .route("/stream", get(stream_handler))
        .route("/config", get(config_handler))
        .route("/health", get(health_handler))
        .with_state(state.clone())
        .layer(
            CorsLayer::new()
                .allow_methods([Method::GET])
                .allow_origin(Any)
                .allow_headers(Any),
        );

    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind to {}", addr))?;

    tracing::info!(%addr, "Backend listening");

    let server_shutdown = shutdown_signal();
    
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            server_shutdown.await;
            shutdown.cancel();
        })
        .await
        .context("Server error")
}

async fn stream_handler(State(state): State<AppState>) -> Response {
    let boundary = "frame";
    let mut ticker = interval(state.config.frame_interval());
    let camera = state.camera.clone();
    let shutdown = state.shutdown.clone();

    let stream = async_stream::stream! {
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    break;
                }
                _ = ticker.tick() => {
                    match camera.capture_frame().await {
                        Ok(frame) => {
                            let mut chunk = BytesMut::with_capacity(frame.len() + 128);
                            chunk.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
                            chunk.extend_from_slice(b"Content-Type: image/jpeg\r\n");
                            chunk.extend_from_slice(format!("Content-Length: {}\r\n\r\n", frame.len()).as_bytes());
                            chunk.extend_from_slice(&frame);
                            chunk.extend_from_slice(b"\r\n");
                            yield Ok::<Bytes, Infallible>(chunk.freeze());
                        }
                        Err(err) => {
                            tracing::error!(error = %err, "Camera capture failed");
                            let mut chunk = BytesMut::new();
                            chunk.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
                            chunk.extend_from_slice(b"Content-Type: text/plain\r\n\r\n");
                            chunk.extend_from_slice(b"camera-error\r\n");
                            yield Ok::<Bytes, Infallible>(chunk.freeze());
                        }
                    }
                }
            }
        }
    };

    let headers = AppendHeaders([(
        header::CONTENT_TYPE,
        format!("multipart/x-mixed-replace; boundary={boundary}"),
    )]);
    let body = Body::from_stream(stream);
    (headers, body).into_response()
}

async fn config_handler(State(state): State<AppState>) -> Json<Config> {
    Json(state.config.clone())
}

async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }

    tracing::info!("Shutdown signal received");
}

fn init_tracing() -> anyhow::Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt()
        .with_env_filter(filter)
        .try_init()
        .map_err(|err| anyhow::anyhow!("Failed to initialize tracing subscriber: {err}"))?;
    Ok(())
}

fn build_camera(config: &Config) -> Arc<dyn Camera> {
    #[cfg(target_os = "linux")]
    {
        // Try rpicam-vid (libcamera) first as it is preferred on modern Pi hardware
        match RpiCamCamera::new(
            config.resolution_width,
            config.resolution_height,
            config.frame_rate,
            config.tuning_file.as_deref(),
            config.extra_args.as_deref(),
        ) {
            Ok(rpi_camera) => {
                tracing::info!("Using rpicam-vid (libcamera) for capture");
                return Arc::new(rpi_camera);
            }
            Err(err) => {
                tracing::debug!(error = %err, "rpicam-vid unavailable or failed to start");
            }
        }

        // Fallback to V4L2 if rpicam-vid is not an option
        if let Some(device) = config.camera_device.as_deref() {
            match V4l2Camera::new(
                device,
                config.resolution_width,
                config.resolution_height,
                config.frame_rate,
            ) {
                Ok(real_camera) => {
                    tracing::info!(device, "Using V4L2 camera device");
                    return Arc::new(real_camera);
                }
                Err(err) => {
                    tracing::error!(device, error = %err, "Failed to initialize V4L2 camera");
                }
            }
        } else {
            tracing::warn!("No camera device configured; using mock camera");
        }
    }

    Arc::new(MockCamera::new(
        config.resolution_width,
        config.resolution_height,
    ))
}
