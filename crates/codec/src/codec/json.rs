use std::marker::PhantomData;
use super::{Decoder, Encoder};
use bytes::{Buf, BufMut, BytesMut};
use serde::{Deserialize, Serialize};
use serde_json::Error;

/// A codec for JSON encoding and decoding using serde_json
/// Enc is the type to encode, Dec is the type to decode
/// ```
/// # use futures_util::{stream::TryStreamExt, io::Cursor};
/// use serde::{Serialize, Deserialize};
/// use yz_futures_codec::{codec::Json, Framed};
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
///     let codec = Json::<Something, Something>::new();
///     let mut framed = Framed::new(stream, codec);
///
///     while let Some(s) = framed.try_next().await.unwrap() {
///         println!("{:?}", s.data);
///     }
/// });
/// ```
pub struct Json<Enc, Dec>(PhantomData<(Enc, Dec)>);
impl_phantom!(Json<Enc, Dec>);

/// Decoder impl parses json objects from bytes
impl<Enc, Dec> Decoder for Json<Enc, Dec>
where
    for<'de> Dec: Deserialize<'de> + 'static,
{
    type Item = Dec;
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Build streaming JSON iterator over data
        let de = serde_json::Deserializer::from_slice(&buf);
        let mut iter = de.into_iter::<Dec>();

        // Attempt to fetch an item and generate response
        let res = match iter.next() {
            Some(Ok(v)) => Ok(Some(v)),
            Some(Err(ref e)) if e.is_eof() => Ok(None),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        };

        // Update offset from iterator
        let offset = iter.byte_offset();

        // Advance buffer
        buf.advance(offset);

        res
    }
}

/// Encoder impl encodes object streams to bytes
impl<Enc, Dec> Encoder for Json<Enc, Dec>
where
    Enc: Serialize + 'static,
{
    type Item = Enc;
    type Error = Error;

    fn encode(&mut self, data: Self::Item, buf: &mut BytesMut) -> Result<(), Self::Error> {
        // Encode json
        let j = serde_json::to_string(&data)?;

        // Write to buffer
        buf.reserve(j.len());
        buf.put_slice(&j.as_bytes());

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use bytes::BytesMut;
    use serde::{Deserialize, Serialize};

    use super::Json;
    use crate::{Decoder, Encoder};

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct TestStruct {
        pub name: String,
        pub data: u16,
    }

    #[test]
    fn json_codec_encode_decode() {
        let mut codec = Json::<TestStruct, TestStruct>::new();
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
    fn json_codec_partial_decode() {
        let mut codec = Json::<TestStruct, TestStruct>::new();
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
