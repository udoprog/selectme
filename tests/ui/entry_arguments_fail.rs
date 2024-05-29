fn main() {}

#[selectme::main]
fn main_with_argument(_value: u32) {}

#[selectme::main]
pub(crate) fn non_empty_pub_crate(_value: u32) {}
