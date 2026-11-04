//! Offline Windows CLI fixture: replay only the cassette in the test's working folder.
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};

fn main() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let mut message = String::new();
    input.read_line(&mut message)?;
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    for line in BufReader::new(File::open("cassette.jsonl")?).lines() {
        let line = line?;
        writeln!(output, "{line}")?;
        output.flush()?;
        if line.starts_with("{\"type\":\"control_request\"") {
            let mut answer = String::new();
            if input.read_line(&mut answer)? != 0 {
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("decisions")?
                    .write_all(answer.as_bytes())?;
            }
        }
    }
    Ok(())
}
