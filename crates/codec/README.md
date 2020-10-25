# yz-futures-codec

Utilities for encoding and decoding frames using `async/await`.

Contains adapters to go from streams of bytes, `AsyncRead` and `AsyncWrite`,
to framed streams implementing `Sink` and `Stream`. Framed streams are also known as transports.

[![Latest Version](https://img.shields.io/crates/v/yz-futures-codec.svg)](https://crates.io/crates/yz-futures-codec)
[![Rust Documentation](https://img.shields.io/badge/api-rustdoc-blue.svg)](https://docs.rs/yz-futures-codec)
![LICENSE](https://img.shields.io/badge/license-MIT-blue.svg)

### Example
```rust
use yz_futures_codec::{LinesCodec, Framed};

async fn main() {
    // let stream = ...
    let mut framed = Framed::new(stream, LinesCodec {});

    while let Some(line) = framed.try_next().await.unwrap() {
        println!("{:?}", line);
    }
}
```
