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
    pub video_quality: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera_device: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tuning_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_args: Option<String>,
    #[serde(skip_serializing)]
    pub stream_user: Option<String>,
    #[serde(skip_serializing)]
    pub stream_password: Option<String>,
    pub auth_enabled: bool,
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
            .unwrap_or(30.0);

        if !(1.0..=60.0).contains(&frame_rate) {
            return Err(anyhow!("FRAME_RATE must be between 1 and 60"));
        }

        let resolution_width = env::var("FRAME_WIDTH")
            .ok()
            .map(|raw| raw.parse().context("Invalid FRAME_WIDTH"))
            .transpose()?
            .unwrap_or(3840);

        let resolution_height = env::var("FRAME_HEIGHT")
            .ok()
            .map(|raw| raw.parse().context("Invalid FRAME_HEIGHT"))
            .transpose()?
            .unwrap_or(2160);

        if resolution_width == 0 || resolution_height == 0 {
            return Err(anyhow!(
                "FRAME_WIDTH and FRAME_HEIGHT must be greater than zero"
            ));
        }

        let video_quality = env::var("VIDEO_QUALITY")
            .ok()
            .map(|raw| raw.parse().context("Invalid VIDEO_QUALITY"))
            .transpose()?
            .unwrap_or(80);

        if !(1..=100).contains(&video_quality) {
            return Err(anyhow!("VIDEO_QUALITY must be between 1 and 100"));
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

        let tuning_file = env::var("RPICAM_TUNING_FILE").ok();
        let extra_args = env::var("RPICAM_EXTRA_ARGS").ok();

        let stream_user = env::var("STREAM_USER").ok().filter(|s| !s.trim().is_empty());
        let stream_password = env::var("STREAM_PASSWORD").ok().filter(|s| !s.trim().is_empty());
        let auth_enabled = stream_user.is_some() && stream_password.is_some();

        Ok(Self {
            listen_address,
            port,
            frame_rate,
            resolution_width,
            resolution_height,
            video_quality,
            camera_device,
            tuning_file,
            extra_args,
            stream_user,
            stream_password,
            auth_enabled,
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
