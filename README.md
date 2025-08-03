# debbugs-rs

A Rust client library for the [Debian Bug Tracking System (Debbugs)](https://wiki.debian.org/DebbugsSoapInterface).

This crate provides a simple wrapper around the SOAP API for the Debian bug tracking system, allowing you to query bug reports, search for bugs, and retrieve detailed information programmatically.

## Features

- **Async and blocking interfaces**: Choose between `debbugs::Debbugs` (async) and `debbugs::blocking::Debbugs` (blocking)
- **Comprehensive bug data**: Access bug reports, logs, and metadata
- **Search functionality**: Search bugs by package, status, severity, and more
- **Mail parsing**: Optional mail parsing support with the `mailparse` feature

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
debbugs = "0.1"
```

### Feature Flags

- `blocking` (default): Enables the blocking client interface
- `tokio` (default): Enables the async client interface
- `mailparse` (default): Enables parsing of email messages in bug logs

## Usage

### Async Interface

```rust
use debbugs::Debbugs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Debbugs::default();
    
    // Get the 10 newest bugs
    let bugs = client.newest_bugs(10).await?;
    println!("Latest bugs: {:?}", bugs);
    
    // Get detailed status for specific bugs
    let reports = client.get_status(&[12345, 67890]).await?;
    for report in reports {
        println!("Bug #{}: {}", report.bugnumber, report.subject);
    }
    
    // Search for bugs in a specific package
    let search = debbugs::SearchQuery {
        package: Some("rust-debbugs".to_string()),
        ..Default::default()
    };
    let found_bugs = client.get_bugs(search).await?;
    println!("Found {} bugs in package", found_bugs.len());
    
    Ok(())
}
```

### Blocking Interface

```rust
use debbugs::blocking::Debbugs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Debbugs::default();
    
    // Get the 10 newest bugs
    let bugs = client.newest_bugs(10)?;
    println!("Latest bugs: {:?}", bugs);
    
    // Get bug logs with messages
    let logs = client.get_bug_log(12345)?;
    for log in logs {
        println!("Message: {}", log.header);
    }
    
    Ok(())
}
```

### Custom Server

```rust
use debbugs::Debbugs;

let client = Debbugs::new("https://custom-debbugs.example.com/soap.cgi");
```

## Examples

The repository includes several examples in the `examples/` directory:

- `newest.rs` - Fetch the newest bugs
- `get_status.rs` - Get detailed status for specific bugs
- `get_bugs.rs` - Search for bugs matching criteria
- `get_bug_log.rs` - Retrieve bug logs and messages
- `wnpp_bugs.rs` - Find Work-Needing and Prospective Packages (WNPP) bugs
- `all.rs` - Fetch all bugs (use with caution!)

Run an example with:

```bash
cargo run --example newest --features tokio
```

## Documentation

- [API Documentation](https://docs.rs/debbugs)
- [Debian Debbugs SOAP Interface](https://wiki.debian.org/DebbugsSoapInterface)

## License

Licensed under the Apache License, Version 2.0.
