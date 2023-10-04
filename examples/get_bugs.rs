#[tokio::main]
async fn main() {
    #[cfg(feature = "env_logger")]
    env_logger::init();
    use debbugs::Debbugs;
    let debbugs = Debbugs::default();
    let query = debbugs::SearchQuery {
        package: Some("samba"),
        status: Some(debbugs::BugStatus::Open),
        ..Default::default()
    };
    let report = debbugs.get_bugs(&query).await.unwrap();
    println!("{:#?}", report);
}
