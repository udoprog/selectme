pub(crate) fn main() {
}

/// Failed to parse the first block and recover to parse the second one (and
/// avoid emitting an error for it).
async fn error_recovery() {
    selectme::select! {
        binding => {}
    };

    selectme::select! {
        binding0 {} if
        binding1 = expr => {}
        binding2 {}
    };

    selectme::select! {
        binding0 = {} if
        binding1 = expr => {}
        binding2 = {}
    };

    selectme::select! {
        binding0 = expr0 {} if
        binding1 = expr1 => {}
        binding2 = expr2 {}
    };
}
