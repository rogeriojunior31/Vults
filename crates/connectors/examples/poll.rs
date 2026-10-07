//! One real poll of a connector, printing what it watches.
//! Usage: cargo run -p vults-connectors --example poll
use vults_connectors::all;

#[tokio::main]
async fn main() {
    for c in all() {
        match c.poll().await {
            Ok(snapshot) => {
                println!("{}: {} items", c.id(), snapshot.len());
                for (key, value) in &snapshot {
                    println!(
                        "  {key}  ci={}  {}",
                        value["ci"],
                        value["headline"]
                            .as_str()
                            .or(value["title"].as_str())
                            .unwrap_or("")
                    );
                }
            }
            Err(e) => println!("{}: error: {e}", c.id()),
        }
    }
}
