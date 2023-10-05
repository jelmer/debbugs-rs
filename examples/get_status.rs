#[tokio::main]
async fn main() {
    #[cfg(feature = "env_logger")]
    env_logger::init();
    use debbugs::Debbugs;
    let debbugs = Debbugs::default();
    let reports = debbugs.get_status(&[42343, 10432]).await.unwrap();
    println!("{:#?}", reports);
}
