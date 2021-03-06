use core::iter::Iterator;
use futures_lite::future::block_on;
use futures_util::io::{AsyncWrite, Cursor};
use futures_util::stream;
use std::pin::Pin;
use std::task::{Context, Poll};
use yz_futures_codec::{codec::BytesCodec, codec::Lines, Framed};
use yz_futures_util::sink::SinkExt;

// An AsyncWrite which is always ready and just consumes the data
struct AsyncWriteNull {
    // number of poll_write calls
    pub num_poll_write: usize,

    // size of the last poll_write
    pub last_write_size: usize,
}
impl AsyncWrite for AsyncWriteNull {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.num_poll_write += 1;
        self.last_write_size = buf.len();
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[test]
fn line_write() {
    let curs = Cursor::new(vec![0u8; 16]);
    let mut framer = Framed::new(curs, Lines {});
    block_on(framer.send_unpin("Hello\n")).unwrap();
    block_on(framer.send_unpin("World\n")).unwrap();
    let (curs, _) = framer.release();
    assert_eq!(&curs.get_ref()[0..12], b"Hello\nWorld\n");
    assert_eq!(curs.position(), 12);
}

#[test]
fn line_write_to_eof() {
    let mut buf = [0u8; 16];
    let curs = Cursor::new(&mut buf[..]);
    let mut framer = Framed::new(curs, Lines {});
    let _err = block_on(framer.send_unpin("This will fill up the buffer\n")).unwrap_err();
    let (curs, _) = framer.release();
    assert_eq!(curs.position(), 16);
    assert_eq!(&curs.get_ref()[0..16], b"This will fill u");
}

#[test]
fn send_high_water_mark() {
    // stream will output 999 bytes, 1 at at a time, and will always be ready
    let mut stream = stream::iter((0..999).map(|_| b"\0").map(Ok));

    // sink will eat whatever it receives
    let io = AsyncWriteNull {
        num_poll_write: 0,
        last_write_size: 0,
    };

    // expect two sends
    let mut framer = Framed::new(io, BytesCodec {});
    framer.w_high_water_mark = 500;
    block_on(framer.send_all_unpin(&mut stream)).unwrap();
    let (io, _) = framer.release();
    assert_eq!(io.num_poll_write, 2);
    assert_eq!(io.last_write_size, 499);
}
