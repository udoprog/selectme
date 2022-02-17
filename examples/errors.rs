use tokio::sync::oneshot;

#[selectme::main]
async fn main() {
    let (tx1, mut rx1) = oneshot::channel();
    let (tx2, mut rx2) = oneshot::channel();

    tokio::spawn(async move {
        tx1.send("first").unwrap();
    });

    tokio::spawn(async move {
        tx2.send("second").unwrap();
    });

    let mut a = None;
    let mut b = None;

    while a.is_none() || b.is_none() {
        selectme::select! {
            v1 = (&mut rx1) if a.is_none() => a = Some(v1.unwrap()),
            v2 = (&mut rx2) if b.is_none() => b = Some(v2.unwrap()),
        }
    }

    let res = (a.unwrap(), b.unwrap());

    assert_eq!(res.0, "first");
    assert_eq!(res.1, "second");
}
