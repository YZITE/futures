#![feature(test)]

extern crate test;

use futures_lite::future::block_on;
use futures_util::{io::Cursor, stream::TryStreamExt};
use yz_futures_codec::{codec::Lines, Framed};

#[bench]
fn short(b: &mut test::Bencher) {
    let data = [
        ["a"; 16].join("b"),
        ["b"; 16].join("c"),
        ["c"; 16].join("d"),
    ]
    .join("\n");
    b.iter(|| {
        block_on(async {
            let read = Cursor::new(test::black_box(&data));
            let mut framed = Framed::new(read, Lines);

            framed.try_next().await.unwrap();
            framed.try_next().await.unwrap();
            framed.try_next().await.is_ok()
        })
    })
}

#[bench]
fn medium(b: &mut test::Bencher) {
    let data = [
        ["a"; 128].join("b"),
        ["b"; 128].join("c"),
        ["c"; 128].join("d"),
    ]
    .join("\n");
    b.iter(|| {
        block_on(async {
            let read = Cursor::new(test::black_box(&data));
            let mut framed = Framed::new(read, Lines);

            framed.try_next().await.unwrap();
            framed.try_next().await.unwrap();
            framed.try_next().await.is_ok()
        })
    })
}

#[bench]
fn long(b: &mut test::Bencher) {
    let data = [
        ["a"; 2048].join("b"),
        ["b"; 2048].join("c"),
        ["c"; 2048].join("d"),
    ]
    .join("\n");
    b.iter(|| {
        block_on(async {
            let read = Cursor::new(test::black_box(&data));
            let mut framed = Framed::new(read, Lines);

            framed.try_next().await.unwrap();
            framed.try_next().await.unwrap();
            framed.try_next().await.is_ok()
        })
    })
}
