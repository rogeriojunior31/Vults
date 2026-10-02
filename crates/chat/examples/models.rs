//! A provider's live chat models, with the key saved in the keyring (if any).
//! Usage: cargo run -p vultures-ai-chat --example models -- openrouter|anthropic|ollama|…
use vultures_ai_chat::providers;

#[tokio::main]
async fn main() {
    let id = std::env::args().nth(1).unwrap_or_else(|| "openrouter".into());
    let Some(p) = providers::find(&id) else {
        let ids: Vec<&str> = providers::PROVIDERS.iter().map(|p| p.id).collect();
        eprintln!("unknown provider {id}; one of {}", ids.join(", "));
        std::process::exit(2);
    };
    let key = vultures_ai_secrets::get(p.secret()).ok().flatten();
    match providers::models(p, key.as_deref()).await {
        Ok(models) => {
            for m in &models {
                println!("{m}");
            }
            eprintln!("{} chat models from {}", models.len(), p.label);
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
