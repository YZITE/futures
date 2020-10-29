use futures_lite::future::block_on;
use futures_util::{io::Cursor, stream::StreamExt};
use yz_futures_codec::{codec::Length, Framed};
use yz_futures_util::sink::SinkExt;

#[test]
fn same_msgs_are_received_as_were_sent() {
    let cur = Cursor::new(vec![0; 256]);
    let mut framed = Framed::new(cur, Length::<u64>::new());

    let send_msgs = async {
        framed.send_unpin("msg1").await.unwrap();
        framed.send_unpin("msg2").await.unwrap();
        framed.send_unpin("msg3").await.unwrap();
    };
    block_on(send_msgs);

    let (mut cur, _) = framed.release();
    cur.set_position(0);
    let framed = Framed::new(cur, Length::<u64>::new());

    let recv_msgs = framed
        .take(3)
        .map(|res| res.unwrap())
        .map(|buf| String::from_utf8(buf.to_vec()).unwrap())
        .collect::<Vec<_>>();
    let msgs: Vec<String> = block_on(recv_msgs);

    assert!(msgs == vec!["msg1", "msg2", "msg3"]);
}
