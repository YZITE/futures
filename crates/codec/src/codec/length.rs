use super::{Decoder, Encoder};
use bytes::{Buf, Bytes, BytesMut};
use std::convert::TryFrom;
use std::marker::PhantomData;

/// A simple `Codec` implementation sending your data by prefixing it by its length.
///
/// # Example
///
/// This codec will most likely be used wrapped in another codec like so.
///
/// ```
/// use yz_futures_codec::codec::{Decoder, Encoder, Length, OverflowError};
/// use bytes::{Bytes, BytesMut};
/// use std::io::{Error, ErrorKind};
///
/// pub struct MyStringCodec(Length::<u64>);
///
/// #[derive(Debug, thiserror::Error)]
/// pub enum MyError {
///     #[error("item length overflow")]
///     Overflow,
///
///     #[error("string decoding failed")]
///     StringDecode(#[from] std::string::FromUtf8Error),
/// }
///
/// impl From<OverflowError> for MyError {
///     fn from(_: OverflowError) -> MyError {
///         MyError::Overflow
///     }
/// }
///
/// impl Encoder for MyStringCodec {
///     type Item = String;
///     type Error = MyError;
///
///     fn encode(&mut self, src: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
///         let bytes = Bytes::from(src);
///         self.0.encode(bytes, dst)?;
///         Ok(())
///     }
/// }
///
/// impl Decoder for MyStringCodec {
///     type Item = String;
///     type Error = MyError;
///
///     fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
///         match self.0.decode(src)? {
///             Some(bytes) => {
///                 match String::from_utf8(bytes.to_vec()) {
///                     Ok(string) => Ok(Some(string)),
///                     Err(e) => Err(MyError::StringDecode(e))
///                 }
///             },
///             None => Ok(None),
///         }
///     }
/// }
/// ```
pub struct Length<L>(PhantomData<L>);
impl_phantom!(Length<L>);

/// the error returned if [`Length`] fails
#[derive(Debug, thiserror::Error)]
#[error("length overflow")]
pub struct OverflowError;

impl<L> Length<L> {
    const HEADER_LEN: usize = std::mem::size_of::<L>();
}

/// invariant: `std::mem::size_of::<Self>()` has the same length as the serialization of `Self`
pub trait LengthType {
    /// this method should write the given `x` into the destination buffer
    fn encode(x: usize, dst: &mut BytesMut) -> Result<(), OverflowError>;

    /// this method should decode the length from the buffer
    /// (it shouldn't and can't discard it, tho)
    ///
    /// pre-condition: `src.len() >= std::mem::size_of::<Self>()`
    fn start_decode(src: &[u8]) -> u64;
}

macro_rules! impl_length {
    ($($x:ty => $y:expr),+ $(,)?) => {
        $(
        impl LengthType for $x {
            fn encode(x: usize, dst: &mut BytesMut) -> Result<(), OverflowError> {
                let this = Self::try_from(x).map_err(|_| OverflowError)?;
                dst.extend_from_slice(&Self::to_be_bytes(this));
                Ok(())
            }

            fn start_decode(src: &[u8]) -> u64 {
                let mut len_bytes = [0u8; $y];
                len_bytes.copy_from_slice(&src[..$y]);
                Self::from_be_bytes(len_bytes).into()
            }
        }
        )+
    }
}

impl_length!(u8 => 1, u16 => 2, u32 => 4, u64 => 8);

impl<L: LengthType> Encoder for Length<L> {
    type Item = Bytes;
    type Error = OverflowError;

    fn encode(&mut self, src: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.reserve(Self::HEADER_LEN + src.len());
        L::encode(src.len(), dst)?;
        dst.extend_from_slice(&src);
        Ok(())
    }
}

impl<L: LengthType> Decoder for Length<L> {
    type Item = Bytes;
    type Error = OverflowError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Ok(if src.len() < std::mem::size_of::<L>() {
            None
        } else {
            let len = usize::try_from(L::start_decode(&src)).map_err(|_| OverflowError)?;
            if src.len() - Self::HEADER_LEN >= len {
                // Skip the length header we already read.
                src.advance(Self::HEADER_LEN);
                Some(src.split_to(len).freeze())
            } else {
                None
            }
        })
    }
}

#[derive(Debug)]
pub struct LenSkipAhead {
    remaining: u64,
}

impl super::SkipAheadHandler for LenSkipAhead {
    fn continue_skipping(mut self, src: &[u8]) -> Result<(usize, Option<Self>), ()> {
        use std::convert::TryInto;
        Ok(if usize::try_from(self.remaining).map(|rem| src.len() >= rem) == Ok(true) {
            (self.remaining.try_into().unwrap(), None)
        } else /* src.len() < self.remaining */ {
            self.remaining -= u64::try_from(src.len()).unwrap();
            (src.len(), Some(self))
        })
    }
}

impl<L: LengthType> super::DecoderWithSkipAhead for Length<L> {
    type Handler = LenSkipAhead;

    fn prepare_skip_ahead(&mut self, src: &mut BytesMut) -> Self::Handler {
        assert!(src.len() > std::mem::size_of::<L>());

        let len = L::start_decode(&src);

        // skip the length header we already read.
        src.advance(Self::HEADER_LEN);

        LenSkipAhead {
            remaining: len,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod decode {
        use super::*;

        #[test]
        fn it_returns_bytes_withouth_length_header() {
            use bytes::BufMut;
            let mut codec = Length::<u64>::new();

            let mut src = BytesMut::with_capacity(5);
            src.put(&[0, 0, 0, 0, 0, 0, 0, 3u8, 1, 2, 3, 4][..]);
            let item = codec.decode(&mut src).unwrap();

            assert!(item == Some(Bytes::from(&[1u8, 2, 3][..])));
        }
    }
}
