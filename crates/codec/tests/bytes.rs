use futures_lite::future::block_on;
use futures_util::{io::Cursor, stream::TryStreamExt};
use yz_futures_codec::{codec::BytesCodec, Framed};

#[test]
fn decodes() {
    let mut buf = [0u8; 32];
    let expected = buf.clone();
    let cur = Cursor::new(&mut buf[..]);
    let mut framed = Framed::new(cur, BytesCodec {});

    let read = block_on(framed.try_next()).unwrap().unwrap();
    assert_eq!(&read[..], &expected[..]);

    assert!(block_on(framed.try_next()).unwrap().is_none());
}
