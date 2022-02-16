pub fn main() {
}

/// Regular `select!` does not support that `static` option.
async fn error_default_static() {
    selectme::select! { 
        static;
    };
}

/// Static can only be specified once.
async fn error_multiple_static() {
    selectme::inline! { 
        static;
        static;
    };
}
