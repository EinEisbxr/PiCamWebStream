mod mock;

#[cfg(target_os = "linux")]
mod v4l2;

pub use mock::MockCamera;

#[cfg(target_os = "linux")]
pub use v4l2::V4l2Camera;

use async_trait::async_trait;

#[async_trait]
pub trait Camera: Send + Sync {
    async fn capture_frame(&self) -> anyhow::Result<Vec<u8>>;
}
