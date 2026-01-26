use std::{
    io::Cursor,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use async_trait::async_trait;
use image::{codecs::jpeg::JpegEncoder, ImageBuffer, Rgb};
use v4l::buffer::Type;
use v4l::io::traits::CaptureStream;
use v4l::prelude::*;
use v4l::video::Capture;
use v4l::FourCC;
use tokio::task;

use super::Camera;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PixelFormat {
    Mjpeg,
    Yuyv,
}

pub struct V4l2Camera {
    device: Arc<Mutex<Device>>,
    width: u32,
    height: u32,
    pixel_format: PixelFormat,
}

impl V4l2Camera {
    pub fn new(device_path: &str, width: u32, height: u32, frame_rate: f32) -> Result<Self> {
        let dev = Device::with_path(device_path)
            .with_context(|| format!("Failed to open camera device {device_path}"))?;

        // Try to get current format, or use a default if it fails
        let mut format = match dev.format() {
            Ok(fmt) => fmt,
            Err(err) => {
                tracing::warn!(?err, "Failed to get current camera format, using default");
                v4l::Format::new(width, height, FourCC::new(b"YUYV"))
            }
        };

        format.width = width;
        format.height = height;

        let mut pixel_format = PixelFormat::Mjpeg;
        format.fourcc = FourCC::new(b"MJPG");

        if let Err(err) = dev.set_format(&format) {
            tracing::warn!(?err, "MJPG format unsupported or failed to set, trying YUYV");
            format.fourcc = FourCC::new(b"YUYV");
            dev.set_format(&format)
                .map_err(|e| anyhow::anyhow!("Camera does not support MJPG or YUYV: {}", e))?;
            pixel_format = PixelFormat::Yuyv;
        }

        // Attempt to set the frame rate (interval)
        if let Ok(mut params) = dev.params() {
            params.interval = v4l::Fraction::new(1, frame_rate.max(1.0) as u32);
            if let Err(err) = dev.set_params(&params) {
                tracing::warn!(?err, "Failed to set camera frame rate (expected on some devices like Pi 5)");
            }
        }

        Ok(Self {
            device: Arc::new(Mutex::new(dev)),
            width,
            height,
            pixel_format,
        })
    }
}

#[async_trait]
impl Camera for V4l2Camera {
    async fn capture_frame(&self) -> Result<Vec<u8>> {
        let device = self.device.clone();
        let width = self.width;
        let height = self.height;
        let format = self.pixel_format;

        task::spawn_blocking(move || -> Result<Vec<u8>> {
            let mut dev = device.lock().expect("v4l2 camera lock poisoned");
            
            // Create a stream for a single frame capture. 
            // While slightly slower than keeping a persistent stream, it's more robust
            // against device state issues and easier to manage lifetimes with the Camera trait.
            let mut stream = MmapStream::with_buffers(&mut *dev, Type::VideoCapture, 4)
                .map_err(|e| anyhow::anyhow!("Failed to create mmap stream: {}", e))?;
            
            let (data, _) = stream.next()
                .map_err(|e| anyhow::anyhow!("Failed to capture frame from v4l2 next(): {}", e))?;

            match format {
                PixelFormat::Mjpeg => Ok(data.to_vec()),
                PixelFormat::Yuyv => yuyv_to_jpeg(data, width, height),
            }
        })
        .await
        .expect("spawn_blocking failed")
    }
}

fn yuyv_to_jpeg(frame: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
    let expected_len = (width as usize) * (height as usize) * 2;
    if frame.len() < expected_len {
        anyhow::bail!(
            "YUYV frame length {} smaller than expected {} for resolution {}x{}",
            frame.len(),
            expected_len,
            width,
            height
        );
    }

    let mut rgb = Vec::with_capacity((width as usize) * (height as usize) * 3);
    for chunk in frame.chunks_exact(4) {
        let y0 = chunk[0] as f32;
        let u = chunk[1] as f32 - 128.0;
        let y1 = chunk[2] as f32;
        let v = chunk[3] as f32 - 128.0;

        let (r0, g0, b0) = yuv_to_rgb(y0, u, v);
        let (r1, g1, b1) = yuv_to_rgb(y1, u, v);

        rgb.push(r0);
        rgb.push(g0);
        rgb.push(b0);
        rgb.push(r1);
        rgb.push(g1);
        rgb.push(b1);
    }

    let buffer: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_vec(width, height, rgb)
        .context("Failed to build RGB buffer from YUYV data")?;

    let mut cursor = Cursor::new(Vec::new());
    let mut encoder = JpegEncoder::new_with_quality(&mut cursor, 85);
    encoder
        .encode_image(&buffer)
        .context("Failed to encode YUYV frame to JPEG")?;

    Ok(cursor.into_inner())
}

fn yuv_to_rgb(y: f32, u: f32, v: f32) -> (u8, u8, u8) {
    let r = y + 1.402 * v;
    let g = y - 0.344_136 * u - 0.714_136 * v;
    let b = y + 1.772 * u;

    (
        clamp_u8(r.round()),
        clamp_u8(g.round()),
        clamp_u8(b.round()),
    )
}

fn clamp_u8(value: f32) -> u8 {
    value.max(0.0).min(255.0) as u8
}
