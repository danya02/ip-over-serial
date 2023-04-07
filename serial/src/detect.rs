use std::{path::PathBuf, time::Duration};

use tokio::time::Interval;
use tokio_serial::{SerialPortBuilderExt, SerialStream};

#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

/// Module to deal with detecting and constructing serial port structures.

/// Check currently available serial ports in a loop.
/// Once a new one appears, construct a serial port and return it.
///
/// This function is __not thread-safe__ because two of these running in parallel will detect and open the same serial port.
/// Care should be taken to prevent this.
pub async fn detect_port(int: &mut Interval) -> SerialStream {
    let mut last = available_ports().expect("Failed to list available ports");
    debug!("Looking for new ports from: {last:?}");

    loop {
        int.tick().await;
        let current = available_ports().expect("Failed to list available ports");
        if current != last {
            debug!("New port list: {current:?}");
            if current.len() <= last.len() {
                debug!("Length decreased or unchanged, ignoring");
                last = current;
                continue;
            }

            let new: Vec<&PathBuf> = current.iter().filter(|x| !last.contains(x)).collect();
            if new.len() > 1 {
                panic!("Too many new serial ports detected: {new:?}");
            }
            if new.is_empty() {
                debug!("No new serial ports found, ignoring");
                last = current;
                continue;
            }

            let new_serial: PathBuf = new[0].clone();
            debug!("Found new serial port: {new_serial:?}");

            last = current; // if it turns out we can't use this

            for _ in 0..10 {
                let opened =
                    tokio_serial::new(new_serial.to_str().expect("TTY name not unicode"), 9600)
                        .open_native_async();
                match opened {
                    Ok(port) => {
                        return port;
                    }
                    Err(error) => {
                        error!("Error opening port {new_serial:?}: {error:?}");
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
            }
        }
    }
}

/// List the paths to serial ports currently available.
/// On Unix this lists `/dev/tty*`.
fn available_ports() -> Result<Vec<PathBuf>, std::io::Error> {
    let mut output = vec![];
    for item in std::fs::read_dir("/dev/")? {
        let x = item?;
        if let Some(y) = x.file_name().to_str() {
            if y.starts_with("tty") {
                output.push(x.path());
            }
        }
    }
    Ok(output)
}
