//! Prints Codex's trust status for hooks whose command contains a marker.
//! Usage: cargo run -p vults-agents --example codex-trust [marker]
fn main() {
    let home = std::env::home_dir().unwrap_or_default();
    let hooks_path = home.join(".codex/hooks.json");
    let hooks = vults_agent_config::read_json(&hooks_path).unwrap_or_default();
    let config = std::fs::read_to_string(home.join(".codex/config.toml")).unwrap_or_default();
    let marker = std::env::args()
        .nth(1)
        .unwrap_or_else(|| vults_agents::MARKER.into());
    println!(
        "{:?}",
        vults_agents::codex_trust(&hooks, &hooks_path, &config, &marker)
    );
}
