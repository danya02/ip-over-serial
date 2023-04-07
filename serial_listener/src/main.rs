use std::{io::Write, time::Duration};

use tokio::{io::AsyncReadExt, time::interval};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    eprintln!("Please insert your serial adapter now...");
    let mut interval = interval(Duration::from_secs(1));
    let mut port = serial::detect::detect_port(&mut interval).await;
    eprintln!("Acquired port!");
    port.set_exclusive(true).unwrap();

    let mut buf = [0; 1024];
    loop {
        // First read exactly one byte. If the port closes here, we'll get an error
        let count = port
            .read_exact(&mut buf[0..1])
            .await
            .expect("Error while port read_exact");
        let count = count
            + port
                .read(&mut buf[1..])
                .await
                .expect("Error while port read");
        std::io::stdout()
            .write_all(&buf[0..count])
            .expect("Error while stdout write");
        std::io::stdout().flush().expect("Error while stdout flush");
    }
}
