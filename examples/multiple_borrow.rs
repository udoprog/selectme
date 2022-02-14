use tokio_stream::{self as stream, StreamExt};

async fn doit(values: &mut Vec<i32>, v: i32) {
    values.push(v);
}

#[tokio::main]
async fn main() {
    let mut stream1 = stream::iter(vec![1, 2, 3]);
    let mut stream2 = stream::iter(vec![4, 5, 6]);

    let mut values = vec![];

    loop {
        selectme::select! {
            Some(v) = stream1.next() => doit(&mut values, v).await,
            Some(v) = stream2.next() => doit(&mut values, v).await,
            else => break,
        }
    }

    // No need to sort since `selectme` is fair by default.
    // values.sort();
    assert_eq!(&[1, 2, 3, 4, 5, 6], &values[..]);
}
