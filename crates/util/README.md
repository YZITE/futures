# yz-futures-util

Utilities for working with `yz-futures-sink` using `async/await`.

[![Latest Version](https://img.shields.io/crates/v/yz-futures-codec.svg)](https://crates.io/crates/yz-futures-util)
[![Rust Documentation](https://img.shields.io/badge/api-rustdoc-blue.svg)](https://docs.rs/yz-futures-util)

## License

[Licensed](LICENSE) under Apache License, Version 2.0 (https://www.apache.org/licenses/LICENSE-2.0).

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
licensed as above, without any additional terms or conditions.

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
