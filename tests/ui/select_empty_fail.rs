pub(crate) fn main() {
}

/// Empty selects are not permitted.
async fn empty() {
    selectme::select! {}
}
