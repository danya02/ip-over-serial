use std::time::Duration;

use serial::frame::{SerialEthFrameCodec};
use tokio::time::interval;
use tokio_util::codec::Decoder;

use futures_util::StreamExt;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    eprintln!("Please insert your serial adapter now...");
    let mut interval = interval(Duration::from_secs(1));
    let mut port = serial::detect::detect_port(&mut interval).await;
    eprintln!("Acquired port!");
    port.set_exclusive(true).unwrap();

    let codec = SerialEthFrameCodec {};
    let mut frame_port = codec.framed(port);

    loop {
        let new_frame = frame_port.next().await;
        println!("{new_frame:?}");
    }
}
