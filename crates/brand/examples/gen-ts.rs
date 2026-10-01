//! Writes `ui/src/brand.ts` from the Rust constants.
fn main() -> std::io::Result<()> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/src/brand.ts");
    std::fs::write(path, vultures_ai_brand::typescript())
}
