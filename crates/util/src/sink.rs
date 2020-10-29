use core::pin::Pin;
use core::marker::Unpin;
use core::future::Future;
use core::task::{Context, Poll};
use futures_core::{Stream, ready};
#[doc(no_inline)]
pub use yz_futures_sink::{FlushSink, Sink};

#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Send<'a, Si: ?Sized, Item> {
    sink: Pin<&'a mut Si>,
    item: Option<Item>,
}

impl<Si: ?Sized, Item> Unpin for Send<'_, Si, Item> {}

impl<Si: Sink<Item> + ?Sized, Item> Future for Send<'_, Si, Item> {
    type Output = Result<(), Si::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = Pin::into_inner(self);
        if let Some(item) = this.item.take() {
            match this.sink.as_mut().poll_ready(cx)? {
                Poll::Ready(()) => this.sink.as_mut().start_send(item)?,
                Poll::Pending => {
                    this.item = Some(item);
                    return Poll::Pending;
                }
            }
        }

        // we're done sending the item, but want to block on flushing the
        // sink
        this.sink.as_mut().poll_flush(cx)
    }
}

#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct SendAll<'a, Si, St, O>
where
    Si: ?Sized + FlushSink,
    St: ?Sized + Stream<Item = Result<O, <Si as FlushSink>::Error>>,
{
    sink: Pin<&'a mut Si>,
    stream: Option<Pin<&'a mut St>>,
    buffered: Option<O>,
}

impl<Si, St, O> Unpin for SendAll<'_, Si, St, O>
where
    Si: ?Sized + FlushSink,
    St: ?Sized + Stream<Item = Result<O, <Si as FlushSink>::Error>>,
{}

impl<Si, St, O> SendAll<'_, Si, St, O>
where
    Si: ?Sized + Sink<O>,
    St: ?Sized + Stream<Item = Result<O, <Si as FlushSink>::Error>>,
{
    fn try_start_send(&mut self, cx: &mut Context<'_>, item: O) -> Poll<Result<(), <Si as FlushSink>::Error>> {
        debug_assert!(self.buffered.is_none());
        match self.sink.as_mut().poll_ready(cx)? {
            Poll::Ready(()) => {
                Poll::Ready(self.sink.as_mut().start_send(item))
            }
            Poll::Pending => {
                self.buffered = Some(item);
                Poll::Pending
            }
        }
    }
}

impl<Si, St, O> Future for SendAll<'_, Si, St, O>
where
    Si: ?Sized + Sink<O>,
    St: ?Sized + Stream<Item = Result<O, <Si as FlushSink>::Error>>,
{
    type Output = Result<(), <Si as FlushSink>::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = Pin::into_inner(self);

        // If we've got an item buffered already, we need to write it to the
        // sink before we can do anything else
        if let Some(item) = this.buffered.take() {
            ready!(this.try_start_send(cx, item))?
        }

        while let Some(x) = &mut this.stream {
            match x.as_mut().poll_next(cx)? {
                Poll::Ready(Some(item)) => {
                    ready!(this.try_start_send(cx, item))?
                }
                Poll::Ready(None) => {
                    this.stream = None;
                    ready!(this.sink.as_mut().poll_flush(cx))?;
                    return Poll::Ready(Ok(()));
                }
                Poll::Pending => {
                    ready!(this.sink.as_mut().poll_flush(cx))?;
                    return Poll::Pending;
                }
            }
        }
        Poll::Ready(Ok(()))
    }
}

pub trait SinkExt<Item>: Sink<Item> {
    fn send_unpin(&mut self, item: Item) -> Send<'_, Self, Item>
    where
        Self: Unpin,
    {
        Send {
            sink: Pin::new(self),
            item: Some(item),
        }
    }

    fn send(self: Pin<&mut Self>, item: Item) -> Send<'_, Self, Item> {
        Send {
            sink: self,
            item: Some(item),
        }
    }

    fn send_all_unpin<'a, St>(&'a mut self, st: &'a mut St) -> SendAll<'a, Self, St, Item>
    where
        Self: Unpin,
        St: Stream<Item = Result<Item, Self::Error>> + Unpin,
    {
        SendAll {
            sink: Pin::new(self),
            stream: Some(Pin::new(st)),
            buffered: None,
        }
    }

    fn send_all<'a, St>(self: Pin<&'a mut Self>, st: Pin<&'a mut St>) -> SendAll<'a, Self, St, Item>
    where
        Self: Unpin,
        St: Stream<Item = Result<Item, Self::Error>> + Unpin,
    {
        SendAll {
            sink: self,
            stream: Some(st),
            buffered: None,
        }
    }
}

impl<Item, Si: Sink<Item>> SinkExt<Item> for Si {}
