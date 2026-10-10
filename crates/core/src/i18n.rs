//! User-facing text in English, Brazilian Portuguese, Spanish and Simplified Chinese. Whole
//! sentences per language, never one language's fragments glued into another's, so each can order
//! its words freely. The island and Settings translate their own words (`ui/src/i18n/`).

use serde::{Deserialize, Serialize};

use crate::Activity;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Lang {
    #[default]
    #[serde(rename = "en")]
    En,
    #[serde(rename = "pt-BR")]
    PtBr,
    #[serde(rename = "es")]
    Es,
    #[serde(rename = "zh")]
    Zh,
}

impl Lang {
    /// From a language tag or a POSIX locale (`pt_BR.UTF-8`, `es-AR`, `zh-Hans`): English for any
    /// language not translated.
    pub fn from_code(code: &str) -> Self {
        let code = code.trim().to_ascii_lowercase();
        let lang = code.split(['_', '-', '.', '@']).next().unwrap_or("");
        match lang {
            "pt" => Lang::PtBr,
            "es" => Lang::Es,
            "zh" => Lang::Zh,
            _ => Lang::En,
        }
    }

    /// Its tag: `en`, `pt-BR`, `es`, `zh`.
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::PtBr => "pt-BR",
            Lang::Es => "es",
            Lang::Zh => "zh",
        }
    }
}

/// `Editing main.rs`, or just `Editing`.
pub fn step(lang: Lang, activity: Activity, tool: &str, detail: Option<&str>) -> String {
    let test = activity == Activity::Run && detail.is_some_and(is_test_run);
    let mcp = activity == Activity::Web && tool.starts_with("mcp__");
    let verb = match (lang, activity) {
        (_, Activity::Work) => tool,
        (Lang::En, a) => match a {
            Activity::Read => "Reading",
            Activity::Search => "Searching",
            Activity::Edit => "Editing",
            Activity::Run if test => "Testing",
            Activity::Run => "Running",
            Activity::Web if mcp => "Calling",
            Activity::Web => "Browsing",
            Activity::Plan => "Planning",
            Activity::Subagent => "Delegating",
            _ => "Thinking",
        },
        (Lang::PtBr, a) => match a {
            Activity::Read => "Lendo",
            Activity::Search => "Buscando",
            Activity::Edit => "Editando",
            Activity::Run if test => "Testando",
            Activity::Run => "Executando",
            Activity::Web if mcp => "Chamando",
            Activity::Web => "Navegando",
            Activity::Plan => "Planejando",
            Activity::Subagent => "Delegando",
            _ => "Pensando",
        },
        (Lang::Es, a) => match a {
            Activity::Read => "Leyendo",
            Activity::Search => "Buscando",
            Activity::Edit => "Editando",
            Activity::Run if test => "Probando",
            Activity::Run => "Ejecutando",
            Activity::Web if mcp => "Llamando a",
            Activity::Web => "Navegando",
            Activity::Plan => "Planificando",
            Activity::Subagent => "Delegando",
            _ => "Pensando",
        },
        (Lang::Zh, a) => match a {
            Activity::Read => "正在读取",
            Activity::Search => "正在搜索",
            Activity::Edit => "正在编辑",
            Activity::Run if test => "正在测试",
            Activity::Run => "正在运行",
            Activity::Web if mcp => "正在调用",
            Activity::Web => "正在浏览",
            Activity::Plan => "正在规划",
            Activity::Subagent => "正在委派",
            _ => "正在思考",
        },
    };
    match detail {
        Some(d) => format!("{verb} {d}"),
        None => verb.to_string(),
    }
}

/// A command that runs a test suite: `cargo test -p core`, `npm test`, `pytest -k slow`.
fn is_test_run(command: &str) -> bool {
    const RUNNERS: &[&str] = &[
        "cargo test",
        "cargo nextest",
        "npm test",
        "npm run test",
        "pnpm test",
        "yarn test",
        "bun test",
        "npx vitest",
        "npx jest",
        "npx playwright test",
        "vitest",
        "jest",
        "pytest",
        "python -m pytest",
        "go test",
        "mvn test",
        "gradle test",
        "./gradlew test",
        "make test",
        "ctest",
        "rspec",
        "phpunit",
    ];
    let command = command.trim_start();
    RUNNERS.iter().any(|r| {
        command
            .strip_prefix(r)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with([' ', ':', '…']))
    })
}

/// A step one of the user's Always rules allowed, without a card: `Testing cargo test · always
/// allowed`.
pub fn ruled_step(lang: Lang, step: &str) -> String {
    match lang {
        Lang::En => format!("{step} · always allowed"),
        Lang::PtBr => format!("{step} · sempre permitido"),
        Lang::Es => format!("{step} · siempre permitido"),
        Lang::Zh => format!("{step} · 始终允许"),
    }
}

/// Who a notification is about when nothing names it.
pub fn some_agent(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "An agent",
        Lang::PtBr => "Um agente",
        Lang::Es => "Un agente",
        Lang::Zh => "一个智能体",
    }
}

/// A desktop notification's title: `vults needs you`.
pub fn notice_title(lang: Lang, kind: crate::notify::Kind, who: &str) -> String {
    use crate::notify::Kind;
    match (lang, kind) {
        (Lang::En, Kind::NeedsYou) => format!("{who} needs you"),
        (Lang::En, Kind::Finished) => format!("{who} finished"),
        (Lang::En, Kind::Failed) => format!("{who} stopped on an error"),
        (Lang::En, Kind::Silent) => format!("{who} has gone quiet"),
        (Lang::PtBr, Kind::NeedsYou) => format!("{who} precisa de você"),
        (Lang::PtBr, Kind::Finished) => format!("{who} terminou"),
        (Lang::PtBr, Kind::Failed) => format!("{who} parou com um erro"),
        (Lang::PtBr, Kind::Silent) => format!("{who} ficou em silêncio"),
        (Lang::Es, Kind::NeedsYou) => format!("{who} te necesita"),
        (Lang::Es, Kind::Finished) => format!("{who} terminó"),
        (Lang::Es, Kind::Failed) => format!("{who} se detuvo por un error"),
        (Lang::Es, Kind::Silent) => format!("{who} se quedó en silencio"),
        (Lang::Zh, Kind::NeedsYou) => format!("{who} 需要你"),
        (Lang::Zh, Kind::Finished) => format!("{who} 已完成"),
        (Lang::Zh, Kind::Failed) => format!("{who} 因错误而停止"),
        (Lang::Zh, Kind::Silent) => format!("{who} 没有动静了"),
    }
}

/// The notification's one button, which opens the island.
pub fn open(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "Open",
        Lang::PtBr | Lang::Es => "Abrir",
        Lang::Zh => "打开",
    }
}

/// The tray menu's entries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Menu {
    Chat,
    SetUp,
    Quit,
    Island,
    Panel,
    Quiet,
    Paused,
}

pub fn menu(lang: Lang, item: Menu) -> &'static str {
    use Menu::*;
    match (lang, item) {
        (Lang::En, Chat) => "Chat…",
        (Lang::En, SetUp) => "Set up agents…",
        (Lang::En, Quit) => "Quit",
        (Lang::En, Island) => "Island",
        (Lang::En, Panel) => "Panel",
        (Lang::En, Quiet) => "Quiet",
        (Lang::En, Paused) => "Paused",
        (Lang::PtBr, Chat) => "Chat…",
        (Lang::PtBr, SetUp) => "Configurar agentes…",
        (Lang::PtBr, Quit) => "Sair",
        (Lang::PtBr, Island) => "Ilha",
        (Lang::PtBr, Panel) => "Painel",
        (Lang::PtBr, Quiet) => "Discreto",
        (Lang::PtBr, Paused) => "Pausado",
        (Lang::Es, Chat) => "Chat…",
        (Lang::Es, SetUp) => "Configurar agentes…",
        (Lang::Es, Quit) => "Salir",
        (Lang::Es, Island) => "Isla",
        (Lang::Es, Panel) => "Panel",
        (Lang::Es, Quiet) => "Discreto",
        (Lang::Es, Paused) => "En pausa",
        (Lang::Zh, Chat) => "聊天…",
        (Lang::Zh, SetUp) => "设置智能体…",
        (Lang::Zh, Quit) => "退出",
        (Lang::Zh, Island) => "灵动岛",
        (Lang::Zh, Panel) => "面板",
        (Lang::Zh, Quiet) => "安静",
        (Lang::Zh, Paused) => "已暂停",
    }
}

/// "While you were away: 2 finished, 1 failed, 1 waits for you for 12 min." Each language says
/// its own counts, in its own order; the ones that are zero are left out.
pub fn digest(lang: Lang, d: &crate::away::Digest) -> String {
    let (f, x, w) = (d.finished, d.failed, d.waiting);
    let m = (d.waited.as_secs() / 60).max(1);
    let pick = |n: u32, one: &str, many: &str| if n == 1 { one.to_string() } else { many.to_string() };
    let mut parts: Vec<String> = Vec::new();
    match lang {
        Lang::En => {
            if f > 0 {
                parts.push(format!("{f} finished"));
            }
            if x > 0 {
                parts.push(format!("{x} failed"));
            }
            match w {
                0 => {}
                1 => parts.push(format!("1 waits for you for {m} min")),
                _ => parts.push(format!("{w} wait for you, the first for {m} min")),
            }
            format!("While you were away: {}.", parts.join(", "))
        }
        Lang::PtBr => {
            if f > 0 {
                parts.push(format!("{f} {}", pick(f, "terminou", "terminaram")));
            }
            if x > 0 {
                parts.push(format!("{x} {}", pick(x, "falhou", "falharam")));
            }
            match w {
                0 => {}
                1 => parts.push(format!("1 espera por você há {m} min")),
                _ => parts.push(format!("{w} esperam por você, a primeira há {m} min")),
            }
            format!("Enquanto você estava fora: {}.", parts.join(", "))
        }
        Lang::Es => {
            if f > 0 {
                parts.push(format!("{f} {}", pick(f, "terminó", "terminaron")));
            }
            if x > 0 {
                parts.push(format!("{x} {}", pick(x, "falló", "fallaron")));
            }
            match w {
                0 => {}
                1 => parts.push(format!("1 te espera desde hace {m} min")),
                _ => parts.push(format!("{w} te esperan, la primera desde hace {m} min")),
            }
            format!("Mientras no estabas: {}.", parts.join(", "))
        }
        Lang::Zh => {
            if f > 0 {
                parts.push(format!("{f} 个已完成"));
            }
            if x > 0 {
                parts.push(format!("{x} 个失败"));
            }
            match w {
                0 => {}
                1 => parts.push(format!("1 个已等你 {m} 分钟")),
                _ => parts.push(format!("{w} 个在等你，最早的已等 {m} 分钟")),
            }
            format!("你不在时：{}。", parts.join("，"))
        }
    }
}

/// `6 h 20 min`, `45 min`, `under a minute`.
pub fn duration(lang: Lang, secs: u64) -> String {
    let (h, m) = (secs / 3600, secs % 3600 / 60);
    match (lang, h, m) {
        (Lang::En, 0, 0) => "under a minute".into(),
        (Lang::PtBr, 0, 0) => "menos de um minuto".into(),
        (Lang::Es, 0, 0) => "menos de un minuto".into(),
        (Lang::Zh, 0, 0) => "不到一分钟".into(),
        (Lang::Zh, 0, m) => format!("{m} 分钟"),
        (Lang::Zh, h, 0) => format!("{h} 小时"),
        (Lang::Zh, h, m) => format!("{h} 小时 {m} 分钟"),
        (_, 0, m) => format!("{m} min"),
        (_, h, 0) => format!("{h} h"),
        (_, h, m) => format!("{h} h {m} min"),
    }
}

/// The week in one sentence: "41 turns, 6 h 20 min with your agents, most on site." One whole
/// sentence per case, so a translation can reorder them freely.
pub fn recap(lang: Lang, w: &crate::recap::WeekView) -> String {
    let time = duration(lang, w.active_secs);
    let n = w.turns;
    match (lang, n, w.top_project.as_deref()) {
        (Lang::En, 0, _) => "No agent turns that week.".into(),
        (Lang::En, 1, Some(p)) => format!("1 turn, {time} with your agents, on {p}."),
        (Lang::En, 1, None) => format!("1 turn, {time} with your agents."),
        (Lang::En, n, Some(p)) => format!("{n} turns, {time} with your agents, most on {p}."),
        (Lang::En, n, None) => format!("{n} turns, {time} with your agents."),
        (Lang::PtBr, 0, _) => "Nenhum turno de agente nessa semana.".into(),
        (Lang::PtBr, 1, Some(p)) => format!("1 turno, {time} com os seus agentes, em {p}."),
        (Lang::PtBr, 1, None) => format!("1 turno, {time} com os seus agentes."),
        (Lang::PtBr, n, Some(p)) => format!("{n} turnos, {time} com os seus agentes, a maioria em {p}."),
        (Lang::PtBr, n, None) => format!("{n} turnos, {time} com os seus agentes."),
        (Lang::Es, 0, _) => "Ningún turno de agente esa semana.".into(),
        (Lang::Es, 1, Some(p)) => format!("1 turno, {time} con tus agentes, en {p}."),
        (Lang::Es, 1, None) => format!("1 turno, {time} con tus agentes."),
        (Lang::Es, n, Some(p)) => format!("{n} turnos, {time} con tus agentes, la mayoría en {p}."),
        (Lang::Es, n, None) => format!("{n} turnos, {time} con tus agentes."),
        (Lang::Zh, 0, _) => "那一周没有智能体回合。".into(),
        (Lang::Zh, n, Some(p)) => format!("{n} 个回合，与智能体共 {time}，主要在 {p}。"),
        (Lang::Zh, n, None) => format!("{n} 个回合，与智能体共 {time}。"),
    }
}

/// A connector's news.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum News {
    ReviewRequested,
    ChecksFailed,
    ChecksPassed,
    Approved,
    ChangesRequested,
}

/// A connector alert's title: `Checks failed on main · me/app`.
pub fn news(lang: Lang, news: News, name: &str, branch: Option<&str>) -> String {
    use News::*;
    match (lang, news, branch) {
        (Lang::En, ReviewRequested, _) => format!("Review requested · {name}"),
        (Lang::En, ChecksFailed, Some(b)) => format!("Checks failed on {b} · {name}"),
        (Lang::En, ChecksFailed, None) => format!("Checks failed · {name}"),
        (Lang::En, ChecksPassed, Some(b)) => format!("Checks passed on {b} · {name}"),
        (Lang::En, ChecksPassed, None) => format!("Checks passed · {name}"),
        (Lang::En, Approved, _) => format!("Approved · {name}"),
        (Lang::En, ChangesRequested, _) => format!("Changes requested · {name}"),
        (Lang::PtBr, ReviewRequested, _) => format!("Review pedido · {name}"),
        (Lang::PtBr, ChecksFailed, Some(b)) => format!("Checks falharam em {b} · {name}"),
        (Lang::PtBr, ChecksFailed, None) => format!("Checks falharam · {name}"),
        (Lang::PtBr, ChecksPassed, Some(b)) => format!("Checks passaram em {b} · {name}"),
        (Lang::PtBr, ChecksPassed, None) => format!("Checks passaram · {name}"),
        (Lang::PtBr, Approved, _) => format!("Aprovado · {name}"),
        (Lang::PtBr, ChangesRequested, _) => format!("Mudanças pedidas · {name}"),
        (Lang::Es, ReviewRequested, _) => format!("Revisión solicitada · {name}"),
        (Lang::Es, ChecksFailed, Some(b)) => format!("Checks fallaron en {b} · {name}"),
        (Lang::Es, ChecksFailed, None) => format!("Checks fallaron · {name}"),
        (Lang::Es, ChecksPassed, Some(b)) => format!("Checks pasaron en {b} · {name}"),
        (Lang::Es, ChecksPassed, None) => format!("Checks pasaron · {name}"),
        (Lang::Es, Approved, _) => format!("Aprobado · {name}"),
        (Lang::Es, ChangesRequested, _) => format!("Cambios solicitados · {name}"),
        (Lang::Zh, ReviewRequested, _) => format!("请求审查 · {name}"),
        (Lang::Zh, ChecksFailed, Some(b)) => format!("{b} 上的检查失败 · {name}"),
        (Lang::Zh, ChecksFailed, None) => format!("检查失败 · {name}"),
        (Lang::Zh, ChecksPassed, Some(b)) => format!("{b} 上的检查通过 · {name}"),
        (Lang::Zh, ChecksPassed, None) => format!("检查通过 · {name}"),
        (Lang::Zh, Approved, _) => format!("已批准 · {name}"),
        (Lang::Zh, ChangesRequested, _) => format!("需要修改 · {name}"),
    }
}

/// The Monday card: last week's headline.
pub fn recap_card(lang: Lang, headline: &str) -> String {
    match lang {
        Lang::En => format!("Last week: {headline}"),
        Lang::PtBr => format!("Semana passada: {headline}"),
        Lang::Es => format!("La semana pasada: {headline}"),
        Lang::Zh => format!("上周：{headline}"),
    }
}

/// A quiet bird's notification: it only informs (`crate::silence`).
pub fn silent_body(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "No news for 15 minutes while it works. It may be waiting on something in its terminal.",
        Lang::PtBr => {
            "Sem novidades há 15 minutos enquanto trabalha. Talvez esteja esperando algo no terminal."
        }
        Lang::Es => {
            "Sin novedades desde hace 15 minutos mientras trabaja. Puede que espere algo en su terminal."
        }
        Lang::Zh => "工作中已经 15 分钟没有动静。它可能在终端里等待什么。",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_language_is_read_from_a_tag_or_a_locale() {
        for (code, lang) in [
            ("pt_BR.UTF-8", Lang::PtBr),
            ("pt-PT", Lang::PtBr),
            ("es_AR.utf8", Lang::Es),
            ("ES", Lang::Es),
            ("zh-Hans-CN", Lang::Zh),
            ("zh_TW", Lang::Zh),
            ("en_US", Lang::En),
            ("fr_FR", Lang::En),
            ("C", Lang::En),
            ("", Lang::En),
        ] {
            assert_eq!(Lang::from_code(code), lang, "{code}");
        }
        for lang in [Lang::En, Lang::PtBr, Lang::Es, Lang::Zh] {
            assert_eq!(Lang::from_code(lang.code()), lang);
            assert_eq!(serde_json::to_value(lang).unwrap(), lang.code());
        }
    }

    #[test]
    fn every_language_says_the_step_and_the_week() {
        let cases = [
            (Lang::PtBr, "Editando main.rs", "Testando cargo test"),
            (Lang::Es, "Editando main.rs", "Probando cargo test"),
            (Lang::Zh, "正在编辑 main.rs", "正在测试 cargo test"),
        ];
        for (lang, edit, test) in cases {
            assert_eq!(step(lang, Activity::Edit, "Edit", Some("main.rs")), edit);
            assert_eq!(step(lang, Activity::Run, "Bash", Some("cargo test")), test);
        }
        assert_eq!(duration(Lang::Zh, 6 * 3600 + 20 * 60), "6 小时 20 分钟");
        let d = |f, x, w| crate::away::Digest {
            seq: 1,
            finished: f,
            failed: x,
            waiting: w,
            waited: std::time::Duration::from_secs(12 * 60),
        };
        assert_eq!(
            digest(Lang::PtBr, &d(1, 2, 0)),
            "Enquanto você estava fora: 1 terminou, 2 falharam."
        );
        assert_eq!(
            digest(Lang::Es, &d(2, 0, 1)),
            "Mientras no estabas: 2 terminaron, 1 te espera desde hace 12 min."
        );
        assert_eq!(
            digest(Lang::Zh, &d(0, 0, 3)),
            "你不在时：3 个在等你，最早的已等 12 分钟。"
        );
        assert_eq!(duration(Lang::PtBr, 30), "menos de um minuto");
        assert_eq!(
            news(Lang::En, News::ChecksFailed, "me/app", Some("main")),
            "Checks failed on main · me/app"
        );
        assert_eq!(
            news(Lang::PtBr, News::Approved, "me/app#12", None),
            "Aprovado · me/app#12"
        );
        assert_eq!(
            news(Lang::Zh, News::ChecksPassed, "me/app", Some("main")),
            "main 上的检查通过 · me/app"
        );
    }

    #[test]
    fn a_test_run_and_an_mcp_call_get_their_own_verbs() {
        let run = |cmd| step(Lang::En, Activity::Run, "Bash", Some(cmd));
        assert_eq!(run("cargo test -p core"), "Testing cargo test -p core");
        assert_eq!(run("npm test"), "Testing npm test");
        assert_eq!(run("pytest…"), "Testing pytest…");
        assert_eq!(run("cargo build"), "Running cargo build");
        assert_eq!(run("cargo testify"), "Running cargo testify");
        assert_eq!(step(Lang::En, Activity::Run, "Bash", None), "Running");
        assert_eq!(
            step(
                Lang::En,
                Activity::Web,
                "mcp__github__list_prs",
                Some("github · list_prs")
            ),
            "Calling github · list_prs"
        );
        assert_eq!(
            step(Lang::En, Activity::Web, "WebFetch", Some("a.dev")),
            "Browsing a.dev"
        );
    }
}
