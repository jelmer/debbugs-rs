#[tokio::main]
async fn main() {
    #[cfg(feature = "env_logger")]
    env_logger::init();
    use debbugs::Debbugs;
    let debbugs = Debbugs::default();
    println!("{:?}", debbugs.newest_bugs(10).await);
}
