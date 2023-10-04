#[tokio::main]
async fn main() {
    #[cfg(feature = "env_logger")]
    env_logger::init();
    use debbugs::Debbugs;
    let debbugs = Debbugs::default();
    let report = debbugs.get_bug_log(1000).await.unwrap();
    println!("{:#?}", report);
}
