//! Retrieve detailed status information for specific bugs
//!
//! This example demonstrates how to get comprehensive bug reports
//! for specific bug IDs. The status includes title, severity, package,
//! current state, and much more metadata.
//!
//! Run with: cargo run --example get_status --features tokio

#[tokio::main]
async fn main() {
    #[cfg(feature = "env_logger")]
    env_logger::init();

    use debbugs::Debbugs;

    let debbugs = Debbugs::default();

    // Get detailed status for specific bug IDs
    let bug_ids = [42343, 10432];

    match debbugs.get_status(&bug_ids).await {
        Ok(reports) => {
            println!("Retrieved status for {} bugs:", reports.len());
            for (bug_id, report) in reports {
                println!("\nBug #{}:", bug_id);
                if let Some(subject) = &report.subject {
                    println!("  Subject: {}", subject);
                }
                if let Some(package) = &report.package {
                    println!("  Package: {}", package);
                }
                if let Some(severity) = &report.severity {
                    println!("  Severity: {}", severity);
                }
                if let Some(tags) = &report.tags {
                    println!("  Tags: {}", tags);
                }
            }
        }
        Err(e) => eprintln!("Error fetching bug status: {}", e),
    }
}
