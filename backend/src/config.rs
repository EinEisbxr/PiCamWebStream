use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub listen_address: IpAddr,
    pub port: u16,
    pub frame_rate: f32,
    pub resolution_width: u32,
    pub resolution_height: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera_device: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let listen_address = env::var("BACKEND_HOST")
            .ok()
            .map(|raw| raw.parse().context("Invalid BACKEND_HOST"))
            .transpose()?
            .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));

        let port = env::var("BACKEND_PORT")
            .ok()
            .map(|raw| raw.parse().context("Invalid BACKEND_PORT"))
            .transpose()?
            .unwrap_or(8080);

        let frame_rate = env::var("FRAME_RATE")
            .ok()
            .map(|raw| raw.parse().context("Invalid FRAME_RATE"))
            .transpose()?
            .unwrap_or(12.0);

        if !(1.0..=60.0).contains(&frame_rate) {
            return Err(anyhow!("FRAME_RATE must be between 1 and 60"));
        }

        let resolution_width = env::var("FRAME_WIDTH")
            .ok()
            .map(|raw| raw.parse().context("Invalid FRAME_WIDTH"))
            .transpose()?
            .unwrap_or(1280);

        let resolution_height = env::var("FRAME_HEIGHT")
            .ok()
            .map(|raw| raw.parse().context("Invalid FRAME_HEIGHT"))
            .transpose()?
            .unwrap_or(720);

        if resolution_width == 0 || resolution_height == 0 {
            return Err(anyhow!(
                "FRAME_WIDTH and FRAME_HEIGHT must be greater than zero"
            ));
        }

        let camera_device = env::var("CAMERA_DEVICE")
            .ok()
            .and_then(|value| {
                if value.trim().is_empty() {
                    None
                } else {
                    Some(value)
                }
            })
            .or_else(Self::default_camera_device);

        Ok(Self {
            listen_address,
            port,
            frame_rate,
            resolution_width,
            resolution_height,
            camera_device,
        })
    }

    pub fn frame_interval(&self) -> Duration {
        let rate = self.frame_rate.max(1.0);
        Duration::from_secs_f64(1.0 / rate as f64)
    }

    pub fn listen_socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.listen_address, self.port)
    }

    fn default_camera_device() -> Option<String> {
        #[cfg(target_os = "linux")]
        {
            Some("/dev/video0".to_string())
        }

        #[cfg(not(target_os = "linux"))]
        {
            None
        }
    }
}
