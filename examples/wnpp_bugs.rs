#[tokio::main]
async fn main() {
    #[cfg(feature = "env_logger")]
    env_logger::init();
    use debbugs::Debbugs;
    let debbugs = Debbugs::default();
    let query = debbugs::SearchQuery {
        package: Some("wnpp"),
        ..Default::default()
    };
    for ids in debbugs.get_bugs(&query).await.unwrap().chunks(50) {
        for (id, report) in debbugs.get_status(ids).await.unwrap() {
            println!(
                "{}: {}",
                id,
                report.subject.unwrap_or("<no title>".to_string())
            );
        }
    }
}
