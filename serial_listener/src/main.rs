use std::{time::Duration, io::Write};

use tokio::{time::interval, io::AsyncReadExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    eprintln!("Please insert your serial adapter now...");
    let mut interval = interval(Duration::from_secs(1));
    let mut port = serial::detect::detect_port(&mut interval).await;
    eprintln!("Acquired port!");

    let mut buf = [0; 1];
    loop {
        let count = port.read(&mut buf).await.expect("Error while port read");
        std::io::stdout().write_all(&buf[0..count]).expect("Error while stdout write");
        std::io::stdout().flush().expect("Error while stdout flush");
    }
}
