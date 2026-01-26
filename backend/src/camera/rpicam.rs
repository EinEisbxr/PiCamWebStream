use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use anyhow::Result;
use async_trait::async_trait;
use tokio::process::{Child, Command};
use tokio::io::{AsyncReadExt, AsyncBufReadExt};
use std::process::Stdio;

pub struct RpiCamCamera {
    current_frame: Arc<Mutex<Vec<u8>>>,
    child: Arc<Mutex<Option<Child>>>,
    ready: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
}

impl RpiCamCamera {
    pub fn new(width: u32, height: u32, frame_rate: f32, quality: u32, tuning_file: Option<&str>, extra_args: Option<&str>) -> Result<Self> {
        let current_frame = Arc::new(Mutex::new(Vec::new()));
        let frame_store = current_frame.clone();
        let ready = Arc::new(AtomicBool::new(false));
        let ready_flag = ready.clone();
        let failed = Arc::new(AtomicBool::new(false));
        let failed_flag = failed.clone();

        let width_str = width.to_string();
        let height_str = height.to_string();
        let frame_rate_str = frame_rate.to_string();
        let quality_str = quality.to_string();

        let mut args = vec![
            "--codec", "mjpeg",
            "-t", "0",
            "--width", &width_str,
            "--height", &height_str,
            "--framerate", &frame_rate_str,
            "--quality", &quality_str,
            "-o", "-",
            "-n",
        ];

        if let Some(tf) = tuning_file {
            args.push("--tuning-file");
            args.push(tf);
        }

        let extra_owned: Vec<String> = extra_args
            .map(|ea| ea.split_whitespace().map(|s| s.to_string()).collect())
            .unwrap_or_default();

        for arg in &extra_owned {
            args.push(arg);
        }

        let mut child = Command::new("rpicam-vid")
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(false)
            .spawn()?;

        let mut stdout = child.stdout.take().expect("Failed to open rpicam-vid stdout");
        let stderr = child.stderr.take().expect("Failed to open rpicam-vid stderr");
        let child_handle = Arc::new(Mutex::new(Some(child)));
        let child_handle_inner = child_handle.clone();
        let failed_stderr = failed.clone();

        // Spawn a task to log stderr and detect critical errors
        tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stderr);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break,
                    Ok(_) => {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            // Check for critical errors that indicate camera failure
                            if trimmed.contains("ERROR") || 
                               trimmed.contains("failed to allocate") ||
                               trimmed.contains("dmaHeap allocation failure") {
                                tracing::error!(target: "rpicam-vid", "{}", trimmed);
                                failed_stderr.store(true, Ordering::SeqCst);
                            } else if trimmed.contains("WARN") {
                                tracing::warn!(target: "rpicam-vid", "{}", trimmed);
                            } else {
                                tracing::info!(target: "rpicam-vid", "{}", trimmed);
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        tokio::spawn(async move {
            let mut chunk = [0u8; 131072];
            let mut stream_buffer = Vec::with_capacity(8 * 1024 * 1024);
            let mut first_frame = true;

            loop {
                match stdout.read(&mut chunk).await {
                    Ok(0) => {
                        if first_frame {
                            tracing::error!("rpicam-vid exited without producing any frames");
                            failed_flag.store(true, Ordering::SeqCst);
                        }
                        break;
                    }
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
                                if first_frame {
                                    tracing::info!("rpicam-vid: First frame captured successfully");
                                    ready_flag.store(true, Ordering::SeqCst);
                                    first_frame = false;
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
            child: child_handle,
            ready,
            failed,
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
        // Check if camera has failed
        if self.failed.load(Ordering::SeqCst) {
            anyhow::bail!("rpicam-vid failed to initialize - check resolution and CMA memory allocation");
        }
        
        // Wait for camera to be ready (with timeout)
        if !self.ready.load(Ordering::SeqCst) {
            anyhow::bail!("Camera starting up, waiting for first frame from rpicam-vid");
        }
        
        let lock = self.current_frame.lock().unwrap();
        if lock.is_empty() {
             anyhow::bail!("No frame available from rpicam-vid");
        }
        Ok(lock.clone())
    }

    async fn shutdown(&self) {
        let child_opt = {
            let mut lock = self.child.lock().unwrap();
            lock.take()
        };
        
        if let Some(mut child) = child_opt {
             tracing::info!("Gracefully shutting down rpicam-vid...");
             if let Some(id) = child.id() {
                 let _ = Command::new("kill")
                     .args(&["-s", "TERM", &id.to_string()])
                     .output()
                     .await;
             }
             // Wait for it to exit
             if let Err(_) = tokio::time::timeout(std::time::Duration::from_secs(5), child.wait()).await {
                 tracing::warn!("rpicam-vid did not exit in time, forcing kill");
                 let _ = child.kill().await;
             }
        }
    }
}
