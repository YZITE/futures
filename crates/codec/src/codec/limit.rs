#![allow(missing_docs)]

use super::{Decoder, Encoder};
use bytes::{Buf, BytesMut};

pub trait SkipAheadHandler: Sized + std::fmt::Debug {
    /// This method skips chunks of content until the beginning
    /// of the next message is found.
    /// `Self` may contain additional state, like the an amount
    /// remaining bytes to discard.
    ///
    /// # the return value
    ///
    /// This method must return `Err(())` if the codec
    /// detected that it can't skip ahead (and thus can't recover).
    ///
    /// The first value in `Ok((_, _))` is the amount of bytes
    /// which should be skipped.
    /// The second value in `Ok((_, _))` is the `done` marker
    /// (if None, `continue_skipping` can't and won't be called again).
    fn continue_skipping(self, src: &[u8]) -> Result<(usize, Option<Self>), ()>;
}

/// A `SkipAheadHandler` that's always ready.
impl SkipAheadHandler for () {
    fn continue_skipping(self, _: &[u8]) -> Result<(usize, Option<Self>), ()> {
        Ok((0, None))
    }
}

pub trait DecoderWithSkipAhead: Decoder {
    type Handler: SkipAheadHandler;

    /// This method is used to prepare a recovery from a malformed
    /// message via skip-ahead.
    /// `src` is supposed to be at the start of the next message
    /// (which should be skipped).
    /// This method may deserialize the header of the next message,
    /// but it should not try to buffer the whole message body,
    /// as this may result in an OOM, which this method is supposed to prevent.
    fn prepare_skip_ahead(&mut self, src: &mut BytesMut) -> Self::Handler;
}

/// A simple wrapper `Codec` implementation preventing too big (encoded) frame sizes
/// (to prevent denial-of-service via OOM when decoding, and prevent sending messages
///  which are too big to be handled appropriately).
///
/// NOTE: This implementation does not enforce the limit the hard way.
/// It checks it only after giving the decoder a chance to decode items.
#[derive(Debug)]
pub struct Limit<C: DecoderWithSkipAhead> {
    inner: C,
    max_frame_size: usize,
    skip_ahead_state: Option<<C as DecoderWithSkipAhead>::Handler>,
    decoder_defunct: bool,
}

impl<C> Limit<C>
where
    C: DecoderWithSkipAhead,
{
    pub fn new(inner: C, max_frame_size: usize) -> Self {
        Self {
            inner,
            max_frame_size,
            skip_ahead_state: None,
            decoder_defunct: false,
        }
    }
}

/// The error type used by [`Limit`].
#[derive(Debug, thiserror::Error)]
pub enum LimitError<E: std::error::Error + 'static> {
    #[error("frame size limit exceeded (detected at {0} bytes)")]
    LimitExceeded(usize),

    #[error("codec couldn't recover from invalid / too big frame")]
    Defunct,

    #[error(transparent)]
    Inner(#[from] E),
}

impl<C> Encoder for Limit<C>
where
    C: Encoder + DecoderWithSkipAhead,
{
    type Item = <C as Encoder>::Item;
    type Error = LimitError<<C as Encoder>::Error>;

    fn encode(&mut self, src: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut tmp_dst = dst.split_off(dst.len());
        self.inner.encode(src, &mut tmp_dst)?;

        if tmp_dst.len() > self.max_frame_size {
            return Err(LimitError::LimitExceeded(tmp_dst.len()));
        }

        dst.unsplit(tmp_dst);
        Ok(())
    }
}

impl<C> Decoder for Limit<C>
where
    C: DecoderWithSkipAhead,
{
    type Item = <C as Decoder>::Item;
    type Error = LimitError<<C as Decoder>::Error>;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        while let Some(sas) = self.skip_ahead_state.take() {
            match sas.continue_skipping(&src) {
                Ok((amount, next)) => {
                    self.skip_ahead_state = next;
                    debug_assert!(amount <= src.len());
                    src.advance(amount);
                    debug_assert!(amount != 0 || self.skip_ahead_state.is_none());
                    if src.len() == 0 {
                        return Ok(None);
                    }
                }
                Err(()) => {
                    // skip ahead failed. codec is now defunct
                    self.decoder_defunct = true;
                }
            }
        }

        if self.decoder_defunct {
            src.clear();
            return Err(LimitError::Defunct);
        }
            match self.inner.decode(src) {
                Ok(None) if src.len() > self.max_frame_size => {
                    // prepare skip ahead
                    self.skip_ahead_state = Some(self.inner.prepare_skip_ahead(src));
                    Err(LimitError::LimitExceeded(src.len()))
                }
                Ok(x) => Ok(x),
                Err(x) => Err(LimitError::Inner(x)),
            }
    }
}

// TODO: add tests
/*
#[cfg(test)]
mod tests {
    use super::*;

    mod decode {
        use super::*;

        #[test]
        fn x() {
        }
    }
}
*/
