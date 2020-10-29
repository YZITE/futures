use super::{Decoder, Encoder, EncoderError};
use bytes::{Buf, BufMut, BytesMut};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// A codec for JSON encoding and decoding using serde_cbor
/// Enc is the type to encode, Dec is the type to decode
/// ```
/// # use futures_util::{stream::TryStreamExt, io::Cursor};
/// use serde::{Serialize, Deserialize};
/// use yz_futures_codec::{codec::Cbor, Framed};
/// use yz_futures_util::sink::SinkExt;
///
/// #[derive(Serialize, Deserialize)]
/// struct Something {
///     pub data: u16,
/// }
///
/// futures_lite::future::block_on(async move {
///     # let mut buf = vec![];
///     # let stream = Cursor::new(&mut buf);
///     // let stream = ...
///     let codec = Cbor::<Something, Something>::new();
///     let mut framed = Framed::new(stream, codec);
///
///     while let Some(s) = framed.try_next().await.unwrap() {
///         println!("{:?}", s.data);
///     }
/// });
/// ```
pub struct Cbor<Enc, Dec>(PhantomData<(Enc, Dec)>);
impl_phantom!(Cbor<Enc, Dec>);

/// Decoder impl parses cbor objects from bytes
impl<Enc, Dec> Decoder for Cbor<Enc, Dec>
where
    for<'de> Dec: Deserialize<'de> + 'static,
{
    type Item = Dec;
    type Error = serde_cbor::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Build deserializer
        let mut de = serde_cbor::Deserializer::from_slice(&buf);

        // Attempt deserialization
        let res: Result<Dec, _> = serde::de::Deserialize::deserialize(&mut de);

        // If we ran out before parsing, return none and try again later
        let res = match res {
            Ok(v) => Ok(Some(v)),
            Err(e) if e.is_eof() => Ok(None),
            Err(e) => Err(e),
        };

        // Update offset from iterator
        let offset = de.byte_offset();

        // Advance buffer
        buf.advance(offset);

        res
    }
}

impl<Enc, Dec> super::EncoderError for Cbor<Enc, Dec>
where
    Enc: Serialize + 'static,
{
    type Error = serde_cbor::Error;
}

/// Encoder impl encodes object streams to bytes
impl<Enc, Dec> Encoder<Enc> for Cbor<Enc, Dec>
where
    Enc: Serialize + 'static,
{
    fn encode(&mut self, data: &Enc, buf: &mut BytesMut) -> Result<(), Self::Error> {
        // Encode cbor
        let j = serde_cbor::to_vec(data)?;

        // Write to buffer
        buf.reserve(j.len());
        buf.put_slice(&j);

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use bytes::BytesMut;
    use serde::{Deserialize, Serialize};

    use super::Cbor;
    use crate::{Decoder, Encoder};

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct TestStruct {
        pub name: String,
        pub data: u16,
    }

    #[test]
    fn cbor_codec_encode_decode() {
        let mut codec = Cbor::<TestStruct, TestStruct>::new();
        let mut buff = BytesMut::new();

        let item1 = TestStruct {
            name: "Test name".to_owned(),
            data: 16,
        };
        codec.encode(item1.clone(), &mut buff).unwrap();

        let item2 = codec.decode(&mut buff).unwrap().unwrap();
        assert_eq!(item1, item2);

        assert_eq!(codec.decode(&mut buff).unwrap(), None);

        assert_eq!(buff.len(), 0);
    }

    #[test]
    fn cbor_codec_partial_decode() {
        let mut codec = Cbor::<TestStruct, TestStruct>::new();
        let mut buff = BytesMut::new();

        let item1 = TestStruct {
            name: "Test name".to_owned(),
            data: 34,
        };
        codec.encode(item1.clone(), &mut buff).unwrap();

        let mut start = buff.clone().split_to(4);
        assert_eq!(codec.decode(&mut start).unwrap(), None);

        codec.decode(&mut buff).unwrap().unwrap();

        assert_eq!(buff.len(), 0);
    }
}
