use std::time::Duration;

use rand::random;
use serial::frame::{MacAddress, SerialEthFrame, SerialEthFrameCodec};
use tokio::time::interval;
use tokio_util::codec::Decoder;

use futures_util::SinkExt;
use rand::Rng;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    eprintln!("Please insert your serial adapter now...");
    let mut interval = interval(Duration::from_secs(1));
    let mut port = serial::detect::detect_port(&mut interval).await;
    eprintln!("Acquired port!");
    port.set_exclusive(true).unwrap();

    let my_mac: MacAddress = random();
    eprintln!("My MAC: {my_mac:?}");

    let codec = SerialEthFrameCodec {};
    let mut frame_port = codec.framed(port);

    loop {
        let dst_mac: MacAddress = random();
        let vlan_tag: u32 = random();
        let payload_length: u16 = rand::thread_rng().gen_range(1..200);
        let mut payload = Vec::with_capacity(payload_length as usize);
        for i in 0..payload_length {
            payload.push(64 + (i % 32) as u8);
        }

        let frame = SerialEthFrame::new(&my_mac, &dst_mac, vlan_tag, &payload[..]);
        eprintln!("Sending to {dst_mac:?} with payload {payload_length}");
        frame_port.send(frame).await.expect("Failed to send frame");
        interval.tick().await;
    }
}
