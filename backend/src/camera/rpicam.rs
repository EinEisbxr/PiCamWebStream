use std::sync::{Arc, Mutex};
use anyhow::Result;
use async_trait::async_trait;
use tokio::process::{Child, Command};
use tokio::io::AsyncReadExt;
use std::process::Stdio;

pub struct RpiCamCamera {
    current_frame: Arc<Mutex<Vec<u8>>>,
    child: Arc<Mutex<Option<Child>>>,
}

impl RpiCamCamera {
    pub fn new(width: u32, height: u32, frame_rate: f32, tuning_file: Option<&str>, extra_args: Option<&str>) -> Result<Self> {
        let current_frame = Arc::new(Mutex::new(Vec::new()));
        let frame_store = current_frame.clone();

        let width_str = width.to_string();
        let height_str = height.to_string();
        let frame_rate_str = frame_rate.to_string();

        let mut args = vec![
            "--codec", "mjpeg",
            "-t", "0",
            "--width", &width_str,
            "--height", &height_str,
            "--framerate", &frame_rate_str,
            "-o", "-",
            "-n",
        ];

        if let Some(tf) = tuning_file {
            args.push("--tuning-file");
            args.push(tf);
        }

        let mut extra_owned = Vec::new();
        if let Some(ea) = extra_args {
            extra_owned = ea.split_whitespace().map(|s| s.to_string()).collect();
            for arg in &extra_owned {
                args.push(arg);
            }
        }

        let mut child = Command::new("rpicam-vid")
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let mut stdout = child.stdout.take().expect("Failed to open rpicam-vid stdout");
        let child_handle = Arc::new(Mutex::new(Some(child)));
        let child_handle_inner = child_handle.clone();

        tokio::spawn(async move {
            let mut chunk = [0u8; 131072];
            let mut stream_buffer = Vec::with_capacity(8 * 1024 * 1024);

            loop {
                match stdout.read(&mut chunk).await {
                    Ok(0) => break,
                    Ok(n) => {
                        stream_buffer.extend_from_slice(&chunk[..n]);

                        while let Some(start) = find_mjpeg_start(&stream_buffer) {
                            if start > 0 {
                                stream_buffer.drain(..start);
                            }
                            
                            if let Some(end) = find_mjpeg_end(&stream_buffer) {
                                let frame_end = end + 2;
                                let frame = stream_buffer[..frame_end].to_vec();
                                {
                                    let mut lock = frame_store.lock().unwrap();
                                    *lock = frame;
                                }
                                stream_buffer.drain(..frame_end);
                            } else {
                                break;
                            }
                        }

                        if stream_buffer.len() > 32 * 1024 * 1024 {
                            stream_buffer.clear();
                        }
                    }
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::Interrupted { 
                            continue;
                        }
                        tracing::error!(error = %e, "Error reading from rpicam-vid stdout");
                        break;
                    }
                }
            }
            
            // Clean up when the loop exits
            let child = if let Ok(mut lock) = child_handle_inner.lock() {
                lock.take()
            } else {
                None
            };
            if let Some(mut c) = child {
                let _ = c.kill().await;
            }
        });

        Ok(Self { 
            current_frame,
            child: child_handle
        })
    }
}

impl Drop for RpiCamCamera {
    fn drop(&mut self) {
        // We try to kill the child process immediately when the camera is dropped.
        // This is tricky from a sync Drop because kill() on tokio::process::Child is async.
        // However, we can use the std handle if we had it, or just use a sync kill.
        if let Ok(mut lock) = self.child.lock() {
            if let Some(mut child) = lock.take() {
                // Best effort kill
                let _ = child.start_kill();
            }
        }
    }
}

fn find_mjpeg_start(data: &[u8]) -> Option<usize> {
    data.windows(2).position(|w| w == [0xFF, 0xD8])
}

fn find_mjpeg_end(data: &[u8]) -> Option<usize> {
    data.windows(2).position(|w| w == [0xFF, 0xD9])
}

#[async_trait]
impl super::Camera for RpiCamCamera {
    async fn capture_frame(&self) -> Result<Vec<u8>> {
        let lock = self.current_frame.lock().unwrap();
        if lock.is_empty() {
             anyhow::bail!("No frame captured yet from rpicam-vid");
        }
        Ok(lock.clone())
    }
}
