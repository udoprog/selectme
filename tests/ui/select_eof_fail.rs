pub fn main() {
}

/// Various failures caused by input suddenly ending.
async fn error_eof() {
    selectme::select! { _ };
    selectme::select! { binding };
    selectme::select! { binding = };
    selectme::select! { binding = expr };
    selectme::select! { binding = expr => };
}
