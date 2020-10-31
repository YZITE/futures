# yz-futures-codec

Utilities for encoding and decoding frames using `async/await`.

Contains adapters to go from streams of bytes, `AsyncRead` and `AsyncWrite`,
to framed streams implementing `Sink` and `Stream`. Framed streams are also known as transports.

[![Latest Version](https://img.shields.io/crates/v/yz-futures-codec.svg)](https://crates.io/crates/yz-futures-codec)
[![Rust Documentation](https://img.shields.io/badge/api-rustdoc-blue.svg)](https://docs.rs/yz-futures-codec)

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### License of tests

The tests are licensed only under Apache License, Version 2.0,
in order to be compatible with [`futures-micro`](https://github.com/irrustible/futures-micro/issues/5).

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

## Example
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
