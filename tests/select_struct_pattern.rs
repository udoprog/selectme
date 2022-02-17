use std::future::{pending, ready};

/// Various failures caused by input suddenly ending.
#[selectme::test]
async fn select_struct_pattern() {
    struct Foo {
        a: u32,
        b: u32,
    }
    let expr = ready(Foo { a: 1, b: 2 });
    let output = selectme::select! { Foo { a, b } = expr => a + b };
    assert_eq!(output, 1 + 2);
}

/// Various failures caused by input suddenly ending.
#[selectme::test]
async fn select_struct_pattern_block() {
    struct Foo {
        a: u32,
        b: u32,
    }
    let expr1 = pending::<Foo>();
    let expr2 = ready(Foo { a: 1, b: 2 });
    let output =
        selectme::select! { Foo { a, b } = expr1 => a + b, Foo { a, b } = expr2 => a * 4 + b * 4 };
    assert_eq!(output, 12);
}
