#![no_std]
#![forbid(unsafe_code)]

#[doc(no_inline)]
pub use futures_core::ready;

pub mod stream {
    #[doc(no_inline)]
    pub use futures_core::Stream;
}

pub mod sink;
