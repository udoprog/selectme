fn main() {}

#[selectme::main]
fn main_is_not_async() {}

#[selectme::main(foo)]
async fn main_attr_has_unknown_args() {}

#[selectme::main(threadpool::bar)]
async fn main_attr_has_path_args() {}

#[selectme::test]
fn test_is_not_async() {}

#[selectme::test(foo)]
async fn test_attr_has_args() {}

#[selectme::test(foo = 123)]
async fn test_unexpected_attr() {}

#[selectme::test(flavor = 123)]
async fn test_flavor_not_string() {}

#[selectme::test(flavor = "foo")]
async fn test_unknown_flavor() {}

#[selectme::test(flavor = "multi_thread", start_paused = false)]
async fn test_multi_thread_with_start_paused() {}

#[selectme::test(flavor = "multi_thread", worker_threads = "foo")]
async fn test_worker_threads_not_int() {}

#[selectme::test(flavor = "current_thread", worker_threads = 4)]
async fn test_worker_threads_and_current_thread() {}

#[selectme::test]
#[test]
async fn test_has_second_test_attr() {}
