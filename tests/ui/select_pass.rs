pub fn main() {
}

/// Various failures caused by input suddenly ending.
async fn legal_eof() {
    let expr = std::future::pending::<()>();
    selectme::select! { _binding = expr => () };
}

/// Various failures caused by input suddenly ending.
async fn match_struct() {
    struct Foo { a: u32, b: u32 }
    let expr = std::future::ready(Foo { a: 1, b: 2 });
    selectme::select! { Foo { a, b } = expr => a + b };
}

/// Various legal condition forms.
async fn legal_conditions() {
    let expr = std::future::ready(0u32);
    selectme::select! { v = expr if true => v };

    let expr = std::future::ready(0u32);
    selectme::select! { v = expr, if true => v };
}
