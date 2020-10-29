//! Utilities for encoding and decoding frames using `async/await`.
//!
//! Contains adapters to go from streams of bytes, [`AsyncRead`](futures_io::AsyncRead)
//! and [`AsyncWrite`](futures_io::AsyncWrite), to framed streams implementing [`Sink`](yz_futures_sink::Sink) and [`Stream`](futures_core::Stream).
//! Framed streams are also known as `transports`.
//!
//! ```
//! # futures_lite::future::block_on(async move {
//! use futures_util::{TryStreamExt, io::Cursor};
//! use yz_futures_codec::{codec::Lines, Framed, Error};
//!
//! let io = Cursor::new(Vec::new());
//! let mut framed = Framed::new(io, Lines);
//!
//! while let Some(line) = framed.try_next().await? {
//!     dbg!(line);
//! }
//! # Ok::<_, Error<_>>(())
//! # }).unwrap();
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(missing_debug_implementations, rust_2018_idioms)]
#![warn(clippy::all)]

use bytes::Buf;
pub use bytes::{Bytes, BytesMut};
use futures_core::{ready, Stream};
use futures_io::{AsyncRead, AsyncWrite};
use std::task::{Context, Poll};
use std::{io, ops::Deref, pin::Pin};
use yz_futures_sink::{FlushSink, Sink};

/// The generic error enum for this crate.
#[derive(Debug, thiserror::Error)]
pub enum Error<C: std::error::Error + 'static> {
    /// An error which originated in the codec
    #[error("codec error: {0}")]
    Codec(#[source] C),

    /// An error which originated in the underlying I/O object
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Codecs
pub mod codec;
use codec::{Decoder, Encoder, EncoderError};

/// A unified `Stream` and `Sink` interface to an underlying I/O object,
/// using the `Encoder` and `Decoder` traits to encode and decode frames.
///
/// # Example
/// ```
/// use bytes::Bytes;
/// use futures_util::{stream::TryStreamExt, io::Cursor};
/// use yz_futures_codec::{codec::BytesCodec, Framed, Error};
/// use yz_futures_util::sink::SinkExt;
///
/// # futures_lite::future::block_on(async move {
/// let cur = Cursor::new(vec![0u8; 12]);
/// let mut framed = Framed::new(cur, BytesCodec {});
///
/// // Send bytes to `buf` through the `BytesCodec`
/// framed.send_unpin("Hello world!").await?;
///
/// // Release the I/O and codec
/// let (cur, _) = framed.release();
/// assert_eq!(cur.get_ref(), b"Hello world!");
/// # Ok::<_, Error<_>>(())
/// # }).unwrap();
/// ```
// NOTE(zserik): yes, I tried pin-project-lite,
// but it doesn't support structs with field docs.
#[pin_project::pin_project]
#[derive(Debug)]
pub struct Framed<T, U> {
    #[pin]
    inner: T,

    /// the codec used to encode and decode frames
    pub codec: U,

    // write
    w_buffer: BytesMut,
    /// The high-water mark for writes, in bytes
    ///
    /// The send *high-water mark* prevents the `Sink` part
    /// from accepting additional messages to send when its
    /// buffer exceeds this length, in bytes. Attempts to enqueue
    /// additional messages will be deferred until progress is
    /// made on the underlying `AsyncWrite`. This applies
    /// back-pressure on fast senders and prevents unbounded
    /// buffer growth.
    ///
    /// The default high-water mark is 2^17 bytes. Applications
    /// which desire low latency may wish to reduce this value.
    /// There is little point to increasing this value beyond
    /// your socket's `SO_SNDBUF` size. On linux, this defaults
    /// to 212992 bytes but is user-adjustable.
    pub w_high_water_mark: usize,

    // read
    r_buffer: BytesMut,
}

impl<T, U> Deref for Framed<T, U> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

const INITIAL_CAPACITY: usize = 8 * 1024;

impl<T, U> Framed<T, U> {
    /// Creates a new `Framed` transport with the given codec.
    /// A codec is a type which implements `Decoder` and `Encoder`.
    pub fn new(inner: T, codec: U) -> Self {
        Self {
            inner,
            codec,

            w_buffer: BytesMut::with_capacity(INITIAL_CAPACITY),

            // 2^17 bytes, which is slightly over 60% of the default
            // TCP send buffer size (SO_SNDBUF)
            w_high_water_mark: 131072,

            r_buffer: BytesMut::with_capacity(INITIAL_CAPACITY),
        }
    }

    /// Release the I/O and Codec
    pub fn release(self) -> (T, U) {
        (self.inner, self.codec)
    }

    /// Consumes the `Framed`, returning its underlying I/O stream.
    ///
    /// Note that care should be taken to not tamper with the underlying stream
    /// of data coming in as it may corrupt the stream of frames otherwise
    /// being worked with.
    pub fn into_inner(self) -> T {
        self.release().0
    }

    /// Returns a mutable reference to the underlying I/O stream.
    ///
    /// Note that care should be taken to not tamper with the underlying stream
    /// of data coming in as it may corrupt the stream of frames otherwise
    /// being worked with.
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Returns a reference to the read buffer.
    pub fn read_buffer(&self) -> &BytesMut {
        &self.r_buffer
    }
}

impl<T: AsyncRead, U: Decoder> Stream for Framed<T, U> {
    type Item = Result<U::Item, Error<U::Error>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        let mut buf = [0u8; INITIAL_CAPACITY];
        let mut ended = false;

        loop {
            match this
                .codec
                .decode(&mut this.r_buffer)
                .map_err(Error::Codec)?
            {
                Some(item) => return Poll::Ready(Some(Ok(item))),
                None if ended => {
                    return if this.r_buffer.is_empty() {
                        Poll::Ready(None)
                    } else {
                        match this
                            .codec
                            .decode_eof(&mut this.r_buffer)
                            .map_err(Error::Codec)?
                        {
                            Some(item) => Poll::Ready(Some(Ok(item))),
                            None if this.r_buffer.is_empty() => Poll::Ready(None),
                            None => Poll::Ready(Some(Err(io::Error::new(
                                io::ErrorKind::UnexpectedEof,
                                "bytes remaining in stream",
                            )
                            .into()))),
                        }
                    };
                }
                _ => {
                    let n = ready!(this.inner.as_mut().poll_read(cx, &mut buf))?;
                    this.r_buffer.extend_from_slice(&buf[..n]);
                    ended = n == 0;
                    continue;
                }
            }
        }
    }
}

impl<T: AsyncWrite, U> Framed<T, U> {
    fn poll_flush_until(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        limit: usize,
    ) -> Poll<Result<(), io::Error>> {
        let mut this = self.project();
        let orig_len = this.w_buffer.len();

        while this.w_buffer.len() > limit {
            let num_write = ready!(this.inner.as_mut().poll_write(cx, &this.w_buffer))?;

            if num_write == 0 {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "FramedWrite: end of input",
                )));
            }

            this.w_buffer.advance(num_write);
        }

        if orig_len != this.w_buffer.len() {
            this.inner.poll_flush(cx)
        } else {
            Poll::Ready(Ok(()))
        }
    }
}

impl<T, U> FlushSink for Framed<T, U>
where
    T: AsyncWrite,
    U: EncoderError,
{
    type Error = Error<U::Error>;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let high_water_mark = self.w_high_water_mark - 1;
        self.poll_flush_until(cx, high_water_mark)
            .map_err(Into::into)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_flush_until(cx, 0).map_err(Into::into)
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().poll_flush(cx))?;
        self.project().inner.poll_close(cx).map_err(Into::into)
    }
}

impl<'a, Item, T, U> Sink<&'a Item> for Framed<T, U>
where
    Item: ?Sized,
    T: AsyncWrite,
    U: Encoder<Item>,
{
    fn start_send(self: Pin<&mut Self>, item: &'a Item) -> Result<(), Self::Error> {
        let this = self.project();
        this.codec.encode(item, this.w_buffer).map_err(Error::Codec)
    }
}
