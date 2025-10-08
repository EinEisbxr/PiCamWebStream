use std::{
    io::Cursor,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use async_trait::async_trait;
use image::{codecs::jpeg::JpegEncoder, ImageBuffer, Rgb};
use rscam::{self, Config as V4l2Config};
use tokio::task;

use super::Camera;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PixelFormat {
    Mjpeg,
    Yuyv,
}

pub struct V4l2Camera {
    camera: Arc<Mutex<rscam::Camera>>,
    width: u32,
    height: u32,
    pixel_format: PixelFormat,
}

impl V4l2Camera {
    pub fn new(device: &str, width: u32, height: u32, frame_rate: f32) -> Result<Self> {
        let mut camera = rscam::Camera::new(device)
            .with_context(|| format!("Failed to open camera device {device}"))?;

        let fps = frame_rate.max(1.0).round() as u32;
        let resolution = (width, height);

        let mut pixel_format = PixelFormat::Mjpeg;

        match camera.start(&V4l2Config {
            interval: (1, fps.max(1)),
            resolution,
            format: *b"MJPG",
            ..Default::default()
        }) {
            Ok(()) => {}
            Err(err) => {
                tracing::warn!(?resolution, fps, device, error = %err, "MJPG format unsupported, falling back to YUYV");
                let second_attempt = camera.start(&V4l2Config {
                    interval: (1, fps.max(1)),
                    resolution,
                    format: *b"YUYV",
                    ..Default::default()
                });

                match second_attempt {
                    Ok(()) => {
                        pixel_format = PixelFormat::Yuyv;
                    }
                    Err(second_err) => {
                        return Err(anyhow::anyhow!(
                            "Failed to configure camera for MJPG ({err}) and YUYV ({second_err})"
                        ));
                    }
                }
            }
        }

        Ok(Self {
            camera: Arc::new(Mutex::new(camera)),
            width,
            height,
            pixel_format,
        })
    }
}

#[async_trait]
impl Camera for V4l2Camera {
    async fn capture_frame(&self) -> Result<Vec<u8>> {
        let camera = self.camera.clone();
        let width = self.width;
        let height = self.height;
        let format = self.pixel_format;

        task::spawn_blocking(move || {
            let mut camera = camera.lock().expect("v4l2 camera lock poisoned");
            let frame = camera
                .capture()
                .context("Failed to capture frame from v4l2 camera")?;

            match format {
                PixelFormat::Mjpeg => Ok(frame.to_vec()),
                PixelFormat::Yuyv => yuyv_to_jpeg(&frame, width, height),
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
