#[tokio::main]
async fn main() {
    #[cfg(feature = "env_logger")]
    env_logger::init();
    use debbugs::Debbugs;
    let debbugs = Debbugs::default();
    let usertags = debbugs
        .get_usertag(
            "debian-science@lists.debian.org",
            &["field..physics", "field..astronomy"],
        )
        .await;
    println!("{:?}", usertags);
}
