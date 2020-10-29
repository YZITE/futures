use ::bytes::BytesMut;

/// Decoding of frames via buffers, for use with [`Framed`](crate::Framed).
pub trait Decoder {
    /// The type of items returned by `decode`
    type Item;
    /// The type of decoding errors.
    type Error: std::error::Error + 'static;

    /// Decode an item from the src `BytesMut` into an item
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error>;

    /// Called when the input stream reaches EOF, signaling a last attempt to decode
    ///
    /// # Notes
    ///
    /// The default implementation of this method invokes the `Decoder::decode` method.
    fn decode_eof(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.decode(src)
    }
}

/// helper trait
pub trait EncoderError {
    /// The type of encoding errors.
    type Error: std::error::Error + 'static;
}

/// Encoding of messages as bytes, for use with [`Framed`](crate::Framed).
///
/// `Item` is the type of items consumed by `encode`
pub trait Encoder<Item: ?Sized>: EncoderError {
    /// Encodes an item into the `BytesMut` provided by dst.
    fn encode(&mut self, item: &Item, dst: &mut BytesMut) -> Result<(), Self::Error>;
}

macro_rules! impl_phantom {
    ($t:ident < $($param:ident),+ >) => {
        impl<$($param),+> $t<$($param),+> {
            #[allow(missing_docs)]
            pub const fn new() -> Self {
                Self(PhantomData)
            }
        }
        impl<$($param),+> ::std::clone::Clone for $t<$($param),+> {
            fn clone(&self) -> Self { Self::new() }
        }
        impl<$($param),+> ::std::fmt::Debug for $t<$($param),+> {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.debug_struct(stringify!($t)).finish()
            }
        }
        impl<$($param),+> ::std::default::Default for $t<$($param),+> {
            fn default() -> Self { Self::new() }
        }
        impl<$($param),+> ::std::cmp::PartialEq for $t<$($param),+> {
            fn eq(&self, _other: &Self) -> bool { true }
        }
    }
}

mod bytes;
pub use self::bytes::BytesCodec;

mod length;
pub use self::length::{Length, OverflowError};

mod lines;
pub use self::lines::Lines;

mod limit;
pub use self::limit::{DecoderWithSkipAhead, Limit, LimitError, SkipAheadHandler};

#[cfg(feature = "json")]
mod json;
#[cfg(feature = "json")]
pub use self::json::Json;

#[cfg(feature = "cbor")]
mod cbor;
#[cfg(feature = "cbor")]
pub use self::cbor::Cbor;
