use super::{Decoder, Encoder};
use bytes::{BufMut, BytesMut};
use memchr::memchr;
use std::convert::Infallible;

/// A simple `Codec` implementation that splits up data into lines.
///
/// ```rust
/// # futures_lite::future::block_on(async move {
/// use futures_util::stream::TryStreamExt; // for lines.try_next()
/// use yz_futures_codec::{Framed, codec::Lines, Error};
///
/// let input = "hello\nworld\nthis\nis\ndog\n".as_bytes();
/// let mut lines = Framed::new(input, Lines);
/// while let Some(line) = lines.try_next().await? {
///     println!("{}", line);
/// }
/// # Ok::<_, Error<_>>(())
/// # }).unwrap();
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Lines;

impl Encoder for Lines {
    type Item = String;
    type Error = Infallible;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.reserve(item.len());
        dst.put(item.as_bytes());
        Ok(())
    }
}

impl Decoder for Lines {
    type Item = String;
    type Error = std::string::FromUtf8Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match memchr(b'\n', src) {
            Some(pos) => {
                let buf = src.split_to(pos + 1);
                String::from_utf8(buf.to_vec()).map(Some)
            }
            _ => Ok(None),
        }
    }
}
