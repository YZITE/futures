use super::{Decoder, Encoder};
use bytes::{Bytes, BytesMut};
use std::convert::Infallible;

/// A simple codec that ships bytes around
///
/// # Example
///
///  ```
/// # futures_lite::future::block_on(async move {
/// use bytes::Bytes;
/// use futures_util::{stream::TryStreamExt, io::Cursor};
/// use yz_futures_codec::{codec::BytesCodec, Framed};
/// use yz_futures_util::sink::SinkExt;
///
/// let mut buf = vec![];
/// // Cursor implements AsyncRead and AsyncWrite
/// let cur = Cursor::new(&mut buf);
/// let mut framed = Framed::new(cur, BytesCodec);
///
/// framed.send_unpin("Hello World!").await?;
///
/// while let Some(bytes) = framed.try_next().await? {
///     dbg!(bytes);
/// }
/// # Ok::<_, yz_futures_codec::Error<_>>(())
/// # }).unwrap();
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BytesCodec;

impl super::EncoderError for BytesCodec {
    type Error = Infallible;
}

impl<Item> Encoder<Item> for BytesCodec
where
    Item: AsRef<[u8]> + ?Sized,
{
    fn encode(&mut self, src: &Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.extend_from_slice(src.as_ref());
        Ok(())
    }
}

impl Decoder for BytesCodec {
    type Item = Bytes;
    type Error = Infallible;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let len = src.len();
        Ok(if len > 0 {
            Some(src.split_to(len).freeze())
        } else {
            None
        })
    }
}
