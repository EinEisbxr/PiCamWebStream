use std::{
    io::Cursor,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use async_trait::async_trait;
use image::{codecs::jpeg::JpegEncoder, ColorType, ImageBuffer};
use tokio::task;

use super::Camera;

#[derive(Debug)]
pub struct MockCamera {
    counter: Arc<Mutex<u64>>,
    width: u32,
    height: u32,
}

impl MockCamera {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            counter: Arc::new(Mutex::new(0)),
            width,
            height,
        }
    }
}

#[async_trait]
impl Camera for MockCamera {
    async fn capture_frame(&self) -> Result<Vec<u8>> {
        let counter = {
            let mut guard = self.counter.lock().expect("mock camera counter poisoned");
            *guard += 1;
            *guard
        };
        let width = self.width;
        let height = self.height;

        let jpeg = task::spawn_blocking(move || generate_frame(width, height, counter))
            .await
            .expect("spawn blocking failed")?;
        Ok(jpeg)
    }
}

fn generate_frame(width: u32, height: u32, counter: u64) -> Result<Vec<u8>> {
    let mut buffer = ImageBuffer::from_fn(width, height, |x, y| {
        let t = counter as f32;
        let xf = x as f32 / width.max(1) as f32;
        let yf = y as f32 / height.max(1) as f32;
        let r = ((xf * 255.0 + t) % 255.0) as u8;
        let g = ((yf * 255.0 + t * 0.5) % 255.0) as u8;
        let b = (((xf + yf) * 127.0 + t * 0.25) % 255.0) as u8;
        image::Rgb([r, g, b])
    });

    for x in (0..width).step_by((width / 10).max(1) as usize) {
        for y in 0..height {
            buffer.put_pixel(x, y, image::Rgb([255, 255, 255]));
        }
    }

    let mut cursor = Cursor::new(Vec::new());
    let mut encoder = JpegEncoder::new_with_quality(&mut cursor, 80);
    encoder.encode(&buffer, width, height, ColorType::Rgb8)?;

    Ok(cursor.into_inner())
}
