//! Fetch the 10 newest bugs from the Debian bug tracking system
//!
//! This example demonstrates how to use the async client to retrieve
//! the most recently reported bugs. The result is a list of bug IDs
//! ordered from newest to oldest.
//!
//! Run with: cargo run --example newest --features tokio

#[tokio::main]
async fn main() {
    #[cfg(feature = "env_logger")]
    env_logger::init();

    use debbugs::Debbugs;

    let debbugs = Debbugs::default();
    match debbugs.newest_bugs(10).await {
        Ok(bugs) => {
            println!("Latest 10 bugs:");
            for bug_id in bugs {
                println!("  Bug #{}", bug_id);
            }
        }
        Err(e) => eprintln!("Error fetching bugs: {}", e),
    }
}
