//! Content-free offline comparison; does not read audio, infer, download or qualify production.
use std::path::Path;
use vults_spike::meeting_eval::{Prediction, Reference, read_json, score};

fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: vults-meeting-eval REFERENCE_JSON PREDICTION_JSON".into());
    }
    let reference: Reference = read_json(Path::new(&args[0]))?;
    let prediction: Prediction = read_json(Path::new(&args[1]))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&score(&reference, &prediction)?).map_err(|e| e.to_string())?
    );
    Ok(())
}
