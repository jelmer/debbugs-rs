//! Search for bugs matching specific criteria
//!
//! This example demonstrates how to search for bugs using the SearchQuery
//! interface. In this case, we're searching for all open bugs in the
//! "samba" package.
//!
//! Run with: cargo run --example get_bugs --features tokio

#[tokio::main]
async fn main() {
    #[cfg(feature = "env_logger")]
    env_logger::init();

    use debbugs::{BugStatus, Debbugs, SearchQuery};

    let debbugs = Debbugs::default();

    // Search for all open bugs in the samba package
    let query = SearchQuery {
        package: Some("samba"),
        status: Some(BugStatus::Open),
        ..Default::default()
    };

    match debbugs.get_bugs(&query).await {
        Ok(bugs) => {
            println!("Found {} open bugs in samba package:", bugs.len());
            for bug_id in bugs.iter().take(10) {
                // Show first 10
                println!("  Bug #{}", bug_id);
            }
            if bugs.len() > 10 {
                println!("  ... and {} more", bugs.len() - 10);
            }
        }
        Err(e) => eprintln!("Error searching for bugs: {}", e),
    }
}
