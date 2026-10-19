//! Explicit developer installation into a chosen private directory. Never run by the app.
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use vults_voice::meeting::assets::{self, Cancel, Gate, Outcome};

struct ExplicitCommand(AtomicUsize);
impl Gate for ExplicitCommand {
    fn allow(&self, _: &'static str) -> bool {
        self.0.fetch_add(1, Ordering::SeqCst) < 12
    }
    fn done(&self, id: &'static str, outcome: Outcome, ms: u64) {
        eprintln!("asset={id} outcome={outcome:?} elapsed_ms={ms}");
    }
}
#[tokio::main]
async fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [command] if command == "review" => println!(
            "{}",
            serde_json::json!({"bundle":assets::ID,"review":assets::review_id(),"downloadBytes":assets::download_bytes(),"runtimeSha256":assets::RUNTIME_SHA256,"appEnabled":false})
        ),
        [command, directory, review] if command == "install" => {
            let bundle = assets::install(
                PathBuf::from(directory),
                review,
                &ExplicitCommand(AtomicUsize::new(0)),
                Cancel::default(),
                |_| {},
            )
            .await?
            .ok_or("installation cancelled")?;
            println!(
                "{}",
                serde_json::json!({"bundle":assets::ID,"directory":bundle.directory(),"review":assets::review_id()})
            );
        }
        _ => return Err("usage: vults-meeting-assets review | install ABSOLUTE_DIRECTORY REVIEW_ID".into()),
    }
    Ok(())
}
