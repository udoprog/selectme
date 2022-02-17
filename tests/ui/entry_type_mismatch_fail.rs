fn main() {}

#[selectme::main]
async fn missing_semicolon_or_return_type() {
    Ok(())
}

#[selectme::main]
async fn missing_return_type() {
    return Ok(());
}

#[selectme::main]
async fn extra_semicolon() -> Result<(), ()> {
    Ok(());
}
