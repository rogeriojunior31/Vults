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

/// A step one of the user's policies allowed, without a card.
pub fn policy_step(lang: Lang, step: &str) -> String {
    match lang {
        Lang::En => format!("{step} · allowed by your policy"),
        Lang::PtBr => format!("{step} · permitido pela sua política"),
        Lang::Es => format!("{step} · permitido por tu política"),
        Lang::Zh => format!("{step} · 你的策略已允许"),
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
    Nest,
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
        (Lang::En, Nest) => "Nest…",
        (Lang::En, SetUp) => "Set up agents…",
        (Lang::En, Quit) => "Quit",
        (Lang::En, Island) => "Island",
        (Lang::En, Panel) => "Panel",
        (Lang::En, Quiet) => "Quiet",
        (Lang::En, Paused) => "Paused",
        (Lang::PtBr, Chat) => "Chat…",
        (Lang::PtBr, Nest) => "Ninho…",
        (Lang::PtBr, SetUp) => "Configurar agentes…",
        (Lang::PtBr, Quit) => "Sair",
        (Lang::PtBr, Island) => "Ilha",
        (Lang::PtBr, Panel) => "Painel",
        (Lang::PtBr, Quiet) => "Discreto",
        (Lang::PtBr, Paused) => "Pausado",
        (Lang::Es, Chat) => "Chat…",
        (Lang::Es, Nest) => "Nido…",
        (Lang::Es, SetUp) => "Configurar agentes…",
        (Lang::Es, Quit) => "Salir",
        (Lang::Es, Island) => "Isla",
        (Lang::Es, Panel) => "Panel",
        (Lang::Es, Quiet) => "Discreto",
        (Lang::Es, Paused) => "En pausa",
        (Lang::Zh, Chat) => "聊天…",
        (Lang::Zh, Nest) => "巢…",
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

/// The daily budget's notification (W9): its title and body. `spent` and `budget` are already
/// written as money (`$4.10`). Spent, Vults starts no new session; the agents running go on.
pub fn budget(lang: Lang, spent_all: bool, spent: &str, budget: &str) -> (String, String) {
    match (lang, spent_all) {
        (Lang::En, false) => (
            "80% of today's budget used".into(),
            format!("The agents have cost {spent} of your {budget} for today."),
        ),
        (Lang::En, true) => (
            "Today's budget is spent".into(),
            format!(
                "The agents have cost {spent} of your {budget}. Vults starts no new session until tomorrow; the ones running go on."
            ),
        ),
        (Lang::PtBr, false) => (
            "80% do orçamento de hoje usado".into(),
            format!("Os agentes já custaram {spent} dos seus {budget} de hoje."),
        ),
        (Lang::PtBr, true) => (
            "O orçamento de hoje acabou".into(),
            format!(
                "Os agentes já custaram {spent} dos seus {budget}. O Vults não inicia nenhuma sessão nova até amanhã; as que estão rodando continuam."
            ),
        ),
        (Lang::Es, false) => (
            "80 % del presupuesto de hoy usado".into(),
            format!("Los agentes ya costaron {spent} de tus {budget} de hoy."),
        ),
        (Lang::Es, true) => (
            "El presupuesto de hoy se agotó".into(),
            format!(
                "Los agentes ya costaron {spent} de tus {budget}. Vults no inicia ninguna sesión nueva hasta mañana; las que están en marcha siguen."
            ),
        ),
        (Lang::Zh, false) => (
            "今日预算已用 80%".into(),
            format!("智能体今天已花费 {spent}，预算为 {budget}。"),
        ),
        (Lang::Zh, true) => (
            "今日预算已用完".into(),
            format!(
                "智能体已花费 {spent}，预算为 {budget}。明天之前 Vults 不会启动新会话；正在运行的会继续。"
            ),
        ),
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

#[derive(Clone, Copy)]
pub(crate) enum ChatText {
    Greeting,
    Morning,
    Afternoon,
    Evening,
    Waiting,
    Failed,
    Working,
    Quiet,
    AskWaiting,
    AskFailed,
    AskSession,
    AskHelp,
    AskRecent,
}

pub(crate) fn chat(lang: Lang, text: ChatText, project: &str, n: usize) -> String {
    use ChatText::*;
    match (lang, text) {
        (Lang::En, Greeting) => "Ask Zeca".into(),
        (Lang::PtBr, Greeting) => "Perguntar ao Zeca".into(),
        (Lang::Es, Greeting) => "Preguntarle a Zeca".into(),
        (Lang::Zh, Greeting) => "问问 Zeca".into(),
        (Lang::En, Morning) => "Good morning".into(),
        (Lang::PtBr, Morning) => "Bom dia".into(),
        (Lang::Es, Morning) => "Buenos días".into(),
        (Lang::Zh, Morning) => "早上好".into(),
        (Lang::En, Afternoon) => "Good afternoon".into(),
        (Lang::PtBr, Afternoon) => "Boa tarde".into(),
        (Lang::Es, Afternoon) => "Buenas tardes".into(),
        (Lang::Zh, Afternoon) => "下午好".into(),
        (Lang::En, Evening) => "Good evening".into(),
        (Lang::PtBr, Evening) => "Boa noite".into(),
        (Lang::Es, Evening) => "Buenas noches".into(),
        (Lang::Zh, Evening) => "晚上好".into(),
        (Lang::En, Waiting) => format!("{project} is waiting for your answer."),
        (Lang::PtBr, Waiting) => format!("{project} está esperando sua resposta."),
        (Lang::Es, Waiting) => format!("{project} está esperando tu respuesta."),
        (Lang::Zh, Waiting) => format!("{project} 正在等待你的答复。"),
        (Lang::En, Failed) => format!("{project} reported a failure."),
        (Lang::PtBr, Failed) => format!("{project} informou uma falha."),
        (Lang::Es, Failed) => format!("{project} informó de un fallo."),
        (Lang::Zh, Failed) => format!("{project} 报告了失败。"),
        (Lang::En, Working) if n == 1 => "One agent is working. What would you like to check?".into(),
        (Lang::PtBr, Working) if n == 1 => "Um agente está trabalhando. O que você quer verificar?".into(),
        (Lang::Es, Working) if n == 1 => "Un agente está trabajando. ¿Qué quieres comprobar?".into(),
        (Lang::En, Working) => format!("{n} agents are working. What would you like to check?"),
        (Lang::PtBr, Working) => format!("{n} agentes estão trabalhando. O que você quer verificar?"),
        (Lang::Es, Working) => format!("{n} agentes están trabajando. ¿Qué quieres comprobar?"),
        (Lang::Zh, Working) => format!("有 {n} 个智能体正在工作。你想查看什么？"),
        (Lang::En, Quiet) => "The flock is quiet. What would you like to do?".into(),
        (Lang::PtBr, Quiet) => "O bando está tranquilo. O que você quer fazer?".into(),
        (Lang::Es, Quiet) => "La bandada está tranquila. ¿Qué quieres hacer?".into(),
        (Lang::Zh, Quiet) => "鸟群很安静。你想做什么？".into(),
        (Lang::En, AskWaiting) => format!("What is {project} waiting for?"),
        (Lang::PtBr, AskWaiting) => format!("O que {project} está esperando?"),
        (Lang::Es, AskWaiting) => format!("¿Qué está esperando {project}?"),
        (Lang::Zh, AskWaiting) => format!("{project} 正在等待什么？"),
        (Lang::En, AskFailed) => format!("Why did {project} fail?"),
        (Lang::PtBr, AskFailed) => format!("Por que {project} falhou?"),
        (Lang::Es, AskFailed) => format!("¿Por qué falló {project}?"),
        (Lang::Zh, AskFailed) => format!("{project} 为什么失败了？"),
        (Lang::En, AskSession) => format!("What is {project} doing now?"),
        (Lang::PtBr, AskSession) => format!("O que {project} está fazendo agora?"),
        (Lang::Es, AskSession) => format!("¿Qué está haciendo {project} ahora?"),
        (Lang::Zh, AskSession) => format!("{project} 现在在做什么？"),
        (Lang::En, AskHelp) => "What can you help me with?".into(),
        (Lang::PtBr, AskHelp) => "Com o que você pode me ajudar?".into(),
        (Lang::Es, AskHelp) => "¿En qué puedes ayudarme?".into(),
        (Lang::Zh, AskHelp) => "你能帮我做什么？".into(),
        (Lang::En, AskRecent) => "What changed recently?".into(),
        (Lang::PtBr, AskRecent) => "O que mudou recentemente?".into(),
        (Lang::Es, AskRecent) => "¿Qué cambió recientemente?".into(),
        (Lang::Zh, AskRecent) => "最近有什么变化？".into(),
    }
}

/// Descriptions for the independent dictation shortcuts shown in the desktop's consent dialog.
pub fn dictation_shortcut(lang: Lang, id: &str) -> Option<&'static str> {
    Some(match (lang, id) {
        (Lang::En, "talk-command") => "Speak a command for Vults without typing into an app",
        (Lang::PtBr, "talk-command") => "Falar um comando para o Vults sem digitar em outro aplicativo",
        (Lang::Es, "talk-command") => "Decir un comando para Vults sin escribir en otra aplicación",
        (Lang::Zh, "talk-command") => "向 Vults 说出命令，而不在应用中输入文字",
        (Lang::En, "talk") => "Dictate text without opening Chat",
        (Lang::PtBr, "talk") => "Ditar texto sem abrir o Chat",
        (Lang::Es, "talk") => "Dictar texto sin abrir el Chat",
        (Lang::Zh, "talk") => "听写文字，不打开聊天",
        (Lang::En, "talk-again") => "Copy the last dictation again",
        (Lang::PtBr, "talk-again") => "Copiar novamente o último ditado",
        (Lang::Es, "talk-again") => "Volver a copiar el último dictado",
        (Lang::Zh, "talk-again") => "再次复制上次听写",
        (Lang::En, "talk-cancel") => "Cancel dictation or Chat voice",
        (Lang::PtBr, "talk-cancel") => "Cancelar o ditado ou a voz do Chat",
        (Lang::Es, "talk-cancel") => "Cancelar el dictado o la voz del Chat",
        (Lang::Zh, "talk-cancel") => "取消听写或聊天语音",
        _ => return None,
    })
}

#[derive(Clone, Copy, Debug)]
pub enum ApplicationFeedback {
    Disabled,
    Missing,
    NoTarget,
    Requested,
    Failed,
}

pub fn voice_application(lang: Lang, status: ApplicationFeedback) -> &'static str {
    use ApplicationFeedback::*;
    match (lang, status) {
        (Lang::En, Disabled) => "Enable application voice commands in Dictation > Command settings first.",
        (Lang::PtBr, Disabled) => {
            "Ative comandos de voz de aplicativos em Ditado > Configurações dos comandos primeiro."
        }
        (Lang::Es, Disabled) => {
            "Activa los comandos de voz de aplicaciones en Dictado > Configuración de comandos primero."
        }
        (Lang::Zh, Disabled) => "请先在听写 > 命令设置中启用应用语音命令。",
        (Lang::En, Missing) => {
            "No installed application matches that name. Use its desktop name or a Dictionary alias."
        }
        (Lang::PtBr, Missing) => {
            "Nenhum aplicativo instalado corresponde a esse nome. Use o nome do desktop ou um apelido do Dicionário."
        }
        (Lang::Es, Missing) => {
            "Ninguna aplicación instalada coincide con ese nombre. Usa su nombre del escritorio o un alias del Diccionario."
        }
        (Lang::Zh, Missing) => "没有匹配此名称的已安装应用。请使用桌面名称或词典别名。",
        (Lang::En, NoTarget) => {
            "No current project or installed application matches that name. Use a desktop name or a Dictionary alias."
        }
        (Lang::PtBr, NoTarget) => {
            "Nenhum projeto atual ou aplicativo instalado corresponde a esse nome. Use o nome do desktop ou um apelido do Dicionário."
        }
        (Lang::Es, NoTarget) => {
            "Ningún proyecto actual o aplicación instalada coincide con ese nombre. Usa un nombre del escritorio o un alias del Diccionario."
        }
        (Lang::Zh, NoTarget) => "没有匹配此名称的当前项目或已安装应用。请使用桌面名称或词典别名。",
        (Lang::En, Requested) => "Application launch requested. Check its window before trying again.",
        (Lang::PtBr, Requested) => {
            "Abertura do aplicativo solicitada. Confira a janela antes de tentar novamente."
        }
        (Lang::Es, Requested) => {
            "Apertura de la aplicación solicitada. Revisa su ventana antes de reintentar."
        }
        (Lang::Zh, Requested) => "已请求启动应用。请先检查应用窗口再重试。",
        (Lang::En, Failed) => {
            "Could not confirm the application launch. It may have changed or timed out; check its window before trying again."
        }
        (Lang::PtBr, Failed) => {
            "Não foi possível confirmar a abertura. O aplicativo pode ter mudado ou excedido o tempo; confira a janela antes de tentar novamente."
        }
        (Lang::Es, Failed) => {
            "No se pudo confirmar la apertura. La aplicación pudo cambiar o agotar el tiempo; revisa su ventana antes de reintentar."
        }
        (Lang::Zh, Failed) => "无法确认应用启动。应用可能已更改或请求超时；请先检查窗口再重试。",
    }
}

#[derive(Clone, Copy, Debug)]
pub enum OutputFeedback {
    Disabled,
    Unavailable,
    Changed,
    Failed,
}

pub fn voice_output_status(lang: Lang, muted: bool, percent: u32) -> String {
    match (lang, muted) {
        (Lang::En, true) => format!("Computer output is muted at {percent}% volume."),
        (Lang::PtBr, true) => format!("A saída do computador está silenciada, com volume em {percent}%."),
        (Lang::Es, true) => format!("La salida del equipo está silenciada, con volumen al {percent}%."),
        (Lang::Zh, true) => format!("电脑输出已静音，音量为 {percent}%。"),
        (Lang::En, false) => format!("Computer output is unmuted at {percent}% volume."),
        (Lang::PtBr, false) => {
            format!("A saída do computador está com som ativado, com volume em {percent}%.")
        }
        (Lang::Es, false) => {
            format!("La salida del equipo tiene sonido activado, con volumen al {percent}%.")
        }
        (Lang::Zh, false) => format!("电脑输出已取消静音，音量为 {percent}%。"),
    }
}

pub fn voice_output_failure(lang: Lang, feedback: OutputFeedback, completed: usize, total: usize) -> String {
    use OutputFeedback::*;
    match (lang, feedback) {
        (Lang::En, Disabled) => {
            "Enable System audio voice commands in Dictation > Command settings first.".into()
        }
        (Lang::PtBr, Disabled) => {
            "Ative Comandos de voz do áudio do sistema em Ditado > Configurações dos comandos primeiro."
                .into()
        }
        (Lang::Es, Disabled) => {
            "Activa Comandos de voz del audio del sistema en Dictado > Configuración de comandos primero."
                .into()
        }
        (Lang::Zh, Disabled) => "请先在听写 > 命令设置中启用系统音频语音命令。".into(),
        (Lang::En, Unavailable) => format!(
            "Computer audio is unavailable; {completed} of {total} actions confirmed. Check pactl and your local audio service."
        ),
        (Lang::PtBr, Unavailable) => format!(
            "Áudio do computador indisponível; {completed} de {total} ações confirmadas. Confira o pactl e o serviço de áudio local."
        ),
        (Lang::Es, Unavailable) => format!(
            "Audio del equipo no disponible; {completed} de {total} acciones confirmadas. Revisa pactl y el servicio de audio local."
        ),
        (Lang::Zh, Unavailable) => {
            format!("电脑音频不可用；已确认 {total} 个操作中的 {completed} 个。请检查 pactl 和本地音频服务。")
        }
        (Lang::En, Changed) => format!(
            "The default output changed; {completed} of {total} actions confirmed. Check the output before speaking again."
        ),
        (Lang::PtBr, Changed) => format!(
            "A saída padrão mudou; {completed} de {total} ações confirmadas. Confira a saída antes de falar novamente."
        ),
        (Lang::Es, Changed) => format!(
            "La salida predeterminada cambió; {completed} de {total} acciones confirmadas. Revísala antes de hablar de nuevo."
        ),
        (Lang::Zh, Changed) => {
            format!("默认输出已更改；已确认 {total} 个操作中的 {completed} 个。请先检查输出再重新说话。")
        }
        (Lang::En, Failed) => format!(
            "Could not confirm computer audio; {completed} of {total} actions confirmed. Check the output before trying again."
        ),
        (Lang::PtBr, Failed) => format!(
            "Não foi possível confirmar o áudio do computador; {completed} de {total} ações confirmadas. Confira a saída antes de tentar novamente."
        ),
        (Lang::Es, Failed) => format!(
            "No se pudo confirmar el audio del equipo; {completed} de {total} acciones confirmadas. Revisa la salida antes de reintentar."
        ),
        (Lang::Zh, Failed) => {
            format!("无法确认电脑音频；已确认 {total} 个操作中的 {completed} 个。请先检查输出再重试。")
        }
    }
}

pub fn voice_media_unavailable(lang: Lang, disabled: bool) -> &'static str {
    match (lang, disabled) {
        (Lang::En, true) => {
            "Enable Media voice commands in Dictation > Command settings, and turn on Media controls."
        }
        (Lang::PtBr, true) => {
            "Ative Comandos de voz de mídia em Ditado > Configurações dos comandos e ligue os controles de mídia."
        }
        (Lang::Es, true) => {
            "Activa Comandos de voz multimedia en Dictado > Configuración de comandos y los controles multimedia."
        }
        (Lang::Zh, true) => "请在听写 > 命令设置中启用媒体语音命令，并开启媒体控制。",
        (Lang::En, false) => "No selected media player. Start playback and check the media controls.",
        (Lang::PtBr, false) => {
            "Nenhum reprodutor selecionado. Inicie a reprodução e confira os controles de mídia."
        }
        (Lang::Es, false) => {
            "No hay reproductor seleccionado. Inicia la reproducción y revisa los controles multimedia."
        }
        (Lang::Zh, false) => "未选择媒体播放器。请开始播放并检查媒体控制。",
    }
}

pub fn voice_external_unavailable(lang: Lang, folder: bool) -> &'static str {
    match (lang, folder) {
        (Lang::En, true) => {
            "The session folder is unavailable; the batch did not run. Check that its absolute path exists."
        }
        (Lang::PtBr, true) => {
            "A pasta da sessão está indisponível; o lote não foi executado. Confira se o caminho absoluto existe."
        }
        (Lang::Es, true) => {
            "La carpeta de la sesión no está disponible; el lote no se ejecutó. Comprueba que la ruta absoluta exista."
        }
        (Lang::Zh, true) => "会话文件夹不可用，批次未执行。请检查其绝对路径是否存在。",
        (Lang::En, false) => {
            "No supported terminal target is available for this session; the batch did not run. Check its terminal integration."
        }
        (Lang::PtBr, false) => {
            "Não há um terminal compatível disponível para esta sessão; o lote não foi executado. Confira a integração do terminal."
        }
        (Lang::Es, false) => {
            "No hay un terminal compatible disponible para esta sesión; el lote no se ejecutó. Revisa la integración del terminal."
        }
        (Lang::Zh, false) => "此会话没有可用的受支持终端，批次未执行。请检查终端集成。",
    }
}

pub fn voice_external_requested(lang: Lang, folder: bool) -> &'static str {
    match (lang, folder) {
        (Lang::En, true) => "Folder open requested. Window readiness is not confirmed.",
        (Lang::PtBr, true) => "Abertura da pasta solicitada. A janela pronta não foi confirmada.",
        (Lang::Es, true) => "Apertura de carpeta solicitada. No se confirmó que la ventana esté lista.",
        (Lang::Zh, true) => "已请求打开文件夹，尚未确认窗口已就绪。",
        (Lang::En, false) => "Terminal focus requested. Window readiness is not confirmed.",
        (Lang::PtBr, false) => "Foco no terminal solicitado. A janela pronta não foi confirmada.",
        (Lang::Es, false) => "Enfoque del terminal solicitado. No se confirmó que la ventana esté lista.",
        (Lang::Zh, false) => "已请求聚焦终端，尚未确认窗口已就绪。",
    }
}

pub fn voice_flock_batch(lang: Lang, success: bool, completed: usize, total: usize) -> String {
    match (lang, success) {
        (Lang::En, true) => format!("Confirmed {completed} of {total} flock and desktop actions."),
        (Lang::PtBr, true) => format!("{completed} de {total} ações do bando e do computador confirmadas."),
        (Lang::Es, true) => format!("{completed} de {total} acciones del grupo y del equipo confirmadas."),
        (Lang::Zh, true) => format!("已确认 {total} 个群组和桌面操作中的 {completed} 个。"),
        (Lang::En, false) => format!(
            "Confirmed {completed} of {total} flock and desktop actions before the batch stopped. Check the result before trying again; the failed action may already have taken effect. No actions were retried or rolled back."
        ),
        (Lang::PtBr, false) => format!(
            "{completed} de {total} ações do bando e do computador confirmadas antes da interrupção do lote. Confira o resultado antes de tentar novamente; a ação com falha pode já ter sido aplicada. Nenhuma ação foi repetida ou revertida."
        ),
        (Lang::Es, false) => format!(
            "{completed} de {total} acciones del grupo y del equipo confirmadas antes de detener el lote. Revisa el resultado antes de reintentar; la acción fallida puede haberse aplicado. No se repitió ni revirtió ninguna acción."
        ),
        (Lang::Zh, false) => format!(
            "批次停止前已确认 {total} 个群组和桌面操作中的 {completed} 个。请检查结果后再试；失败的操作可能已生效。未重试或回滚任何操作。"
        ),
    }
}

pub fn voice_desktop_batch(lang: Lang, success: bool, completed: usize, total: usize) -> String {
    match (lang, success) {
        (Lang::En, true) => format!(
            "Confirmed {completed} of {total} desktop actions. Application confirmations mean launch requests, not ready windows."
        ),
        (Lang::PtBr, true) => format!(
            "{completed} de {total} ações do computador confirmadas. A confirmação de um aplicativo indica o pedido de abertura, não uma janela pronta."
        ),
        (Lang::Es, true) => format!(
            "{completed} de {total} acciones del equipo confirmadas. La confirmación de una aplicación indica la solicitud de apertura, no una ventana lista."
        ),
        (Lang::Zh, true) => {
            format!("已确认 {total} 个桌面操作中的 {completed} 个。应用确认表示已请求启动，并非窗口已就绪。")
        }
        (Lang::En, false) => format!(
            "Confirmed {completed} of {total} desktop actions before the batch stopped. Check command settings and targets before trying again; no actions were retried. Application confirmations mean launch requests."
        ),
        (Lang::PtBr, false) => format!(
            "{completed} de {total} ações do computador confirmadas antes da interrupção do lote. Confira as configurações dos comandos e os destinos antes de tentar novamente; nenhuma ação foi repetida. A confirmação de um aplicativo indica o pedido de abertura."
        ),
        (Lang::Es, false) => format!(
            "{completed} de {total} acciones del equipo confirmadas antes de detener el lote. Revisa la configuración de comandos y los destinos antes de reintentar; no se repitió ninguna acción. La confirmación de una aplicación indica la solicitud de apertura."
        ),
        (Lang::Zh, false) => format!(
            "批次停止前已确认 {total} 个桌面操作中的 {completed} 个。请检查命令设置和目标后再试；未重试任何操作。应用确认表示已请求启动。"
        ),
    }
}

pub fn voice_audio_batch(lang: Lang, success: bool, completed: usize, total: usize) -> String {
    match (lang, success) {
        (Lang::En, true) => format!("Confirmed {completed} of {total} player and computer audio actions."),
        (Lang::PtBr, true) => {
            format!("{completed} de {total} ações de áudio do reprodutor e do computador confirmadas.")
        }
        (Lang::Es, true) => {
            format!("{completed} de {total} acciones de audio del reproductor y del equipo confirmadas.")
        }
        (Lang::Zh, true) => format!("已确认 {total} 个播放器和电脑音频操作中的 {completed} 个。"),
        (Lang::En, false) => format!(
            "Confirmed {completed} of {total} audio actions. The batch stopped because a control is disabled, unavailable, changed or timed out. Check Command settings, the player and computer output before trying again; no actions were retried."
        ),
        (Lang::PtBr, false) => format!(
            "{completed} de {total} ações de áudio confirmadas. O lote parou porque um controle está desligado, indisponível, mudou ou excedeu o tempo. Confira as configurações dos comandos, o reprodutor e a saída do computador antes de tentar novamente; nenhuma ação foi repetida."
        ),
        (Lang::Es, false) => format!(
            "{completed} de {total} acciones de audio confirmadas. El lote se detuvo porque un control está desactivado, no está disponible, cambió o agotó el tiempo. Revisa la configuración de comandos, el reproductor y la salida del equipo antes de reintentar; no se repitió ninguna acción."
        ),
        (Lang::Zh, false) => format!(
            "已确认 {total} 个音频操作中的 {completed} 个。某个控制已关闭、不可用、已更改或超时，因此批次已停止。请检查命令设置、播放器和电脑输出后再试；未重试任何操作。"
        ),
    }
}

pub fn voice_media_failed(lang: Lang, completed: usize, total: usize) -> String {
    match lang {
        Lang::En => format!(
            "Completed {completed} of {total} media actions; the next action failed or timed out. Check the player before trying again."
        ),
        Lang::PtBr => format!(
            "{completed} de {total} ações de mídia concluídas; a próxima falhou ou excedeu o tempo. Confira o reprodutor antes de tentar novamente."
        ),
        Lang::Es => format!(
            "{completed} de {total} acciones multimedia completadas; la siguiente falló o agotó el tiempo. Revisa el reproductor antes de reintentar."
        ),
        Lang::Zh => {
            format!("已完成 {total} 个媒体操作中的 {completed} 个；下一操作失败或超时。请检查播放器后再试。")
        }
    }
}

/// Local command feedback, shared by the dispatcher and its future island presentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceFeedback {
    Done,
    Requested,
    ExecutionFailed,
    InvalidVolume,
    Quoted,
    Consulting,
    SentToChat,
    ChatUnavailable,
    Answer,
    WhichOne,
    NotByVoice,
    NotCommand,
    UnknownName,
    Expired,
    Stale,
    NoContext,
    CannotUndo,
    Unavailable,
    Locked,
    NoWaiting,
    NoUsage,
    NoPullRequests,
}

pub fn voice_feedback(lang: Lang, feedback: VoiceFeedback) -> &'static str {
    use VoiceFeedback::*;
    match (lang, feedback) {
        (Lang::En, Quoted) => "Quoted speech is not executed as a voice command.",
        (Lang::PtBr, Quoted) => "Uma fala entre aspas não é executada como comando de voz.",
        (Lang::Es, Quoted) => "El texto entre comillas no se ejecuta como comando de voz.",
        (Lang::Zh, Quoted) => "引述内容不会作为语音命令执行。",
        (Lang::En, InvalidVolume) => {
            "Use a whole number from 0 to 100 percent, without a plus or minus sign."
        }
        (Lang::PtBr, InvalidVolume) => "Use um volume inteiro entre 0 e 100 por cento, sem sinal.",
        (Lang::Es, InvalidVolume) => "Usa un volumen entero entre 0 y 100 por ciento, sin signo.",
        (Lang::Zh, InvalidVolume) => "请使用 0 到 100 之间不带符号的整数音量百分比。",
        (Lang::En, SentToChat) => "Sent to Zeca. Follow the reply in Chat.",
        (Lang::PtBr, SentToChat) => "Enviado ao Zeca. Acompanhe a resposta no Chat.",
        (Lang::Es, SentToChat) => "Enviado a Zeca. Sigue la respuesta en Chat.",
        (Lang::Zh, SentToChat) => "已发送给 Zeca。请在聊天中查看回复。",
        (Lang::En, ChatUnavailable) => "Could not send to Zeca. Check Chat, finish any draft and try again.",
        (Lang::PtBr, ChatUnavailable) => {
            "Não foi possível enviar ao Zeca. Confira o Chat, conclua o rascunho e tente novamente."
        }
        (Lang::Es, ChatUnavailable) => {
            "No se pudo enviar a Zeca. Revisa Chat, termina el borrador e inténtalo de nuevo."
        }
        (Lang::Zh, ChatUnavailable) => "无法发送给 Zeca。请检查聊天，完成草稿后重试。",
        (Lang::En, Requested) => "Command requested.",
        (Lang::PtBr, Requested) => "Comando solicitado.",
        (Lang::Es, Requested) => "Comando solicitado.",
        (Lang::Zh, Requested) => "命令已请求。",
        (Lang::En, Consulting) => "I'm checking with Claude Code. You can cancel while you wait.",
        (Lang::PtBr, Consulting) => "Estou consultando o Claude Code. Você pode cancelar enquanto espera.",
        (Lang::Es, Consulting) => "Estoy consultando a Claude Code. Puedes cancelar mientras esperas.",
        (Lang::Zh, Consulting) => "正在咨询 Claude Code。等待时可以取消。",
        (Lang::En, ExecutionFailed) => {
            "Could not complete every action. Some changes may already have happened."
        }
        (Lang::PtBr, ExecutionFailed) => {
            "Não foi possível concluir todas as ações. Algumas alterações podem já ter ocorrido."
        }
        (Lang::Es, ExecutionFailed) => {
            "No se pudieron completar todas las acciones. Es posible que algunos cambios ya se hayan realizado."
        }
        (Lang::Zh, ExecutionFailed) => "无法完成所有操作。部分更改可能已经发生。",
        (Lang::En, Done) => "Command completed.",
        (Lang::PtBr, Done) => "Comando concluído.",
        (Lang::Es, Done) => "Comando completado.",
        (Lang::Zh, Done) => "命令已完成。",
        (Lang::En, Answer) => "Here is the local snapshot.",
        (Lang::PtBr, Answer) => "Aqui está o retrato local.",
        (Lang::Es, Answer) => "Aquí está la información local.",
        (Lang::Zh, Answer) => "这是本地快照。",
        (Lang::En, WhichOne) => "Which one? Say one of these names within 90 seconds.",
        (Lang::PtBr, WhichOne) => "Qual deles? Diga um destes nomes em até 90 segundos.",
        (Lang::Es, WhichOne) => "¿Cuál? Di uno de estos nombres en un plazo de 90 segundos.",
        (Lang::Zh, WhichOne) => "哪一个？请在 90 秒内说出其中一个名称。",
        (Lang::En, NotByVoice) => "Voice commands cannot answer cards. Use the card or your terminal.",
        (Lang::PtBr, NotByVoice) => "Comandos de voz não respondem a cards. Use o card ou seu terminal.",
        (Lang::Es, NotByVoice) => "Los comandos de voz no responden tarjetas. Usa la tarjeta o tu terminal.",
        (Lang::Zh, NotByVoice) => "语音命令无法回答卡片。请使用卡片或终端。",
        (Lang::En, NotCommand) => "That is not a voice command.",
        (Lang::PtBr, NotCommand) => "Isso não é um comando de voz.",
        (Lang::Es, NotCommand) => "Eso no es un comando de voz.",
        (Lang::Zh, NotCommand) => "这不是语音命令。",
        (Lang::En, UnknownName) => "No current project or agent matches that name.",
        (Lang::PtBr, UnknownName) => "Nenhum projeto ou agente atual corresponde a esse nome.",
        (Lang::Es, UnknownName) => "Ningún proyecto o agente actual coincide con ese nombre.",
        (Lang::Zh, UnknownName) => "当前没有与该名称匹配的项目或智能体。",
        (Lang::En, Expired) => "That question expired. Say the command again.",
        (Lang::PtBr, Expired) => "Essa pergunta expirou. Diga o comando novamente.",
        (Lang::Es, Expired) => "Esa pregunta caducó. Di el comando de nuevo.",
        (Lang::Zh, Expired) => "该问题已过期。请重新说出命令。",
        (Lang::En, Stale) => "The choices changed. Say the command again.",
        (Lang::PtBr, Stale) => "As opções mudaram. Diga o comando novamente.",
        (Lang::Es, Stale) => "Las opciones cambiaron. Di el comando de nuevo.",
        (Lang::Zh, Stale) => "选项已变化。请重新说出命令。",
        (Lang::En, NoContext) => "There is no recent command to repeat or undo.",
        (Lang::PtBr, NoContext) => "Não há comando recente para repetir ou desfazer.",
        (Lang::Es, NoContext) => "No hay un comando reciente para repetir o deshacer.",
        (Lang::Zh, NoContext) => "没有可重复或撤销的近期命令。",
        (Lang::En, CannotUndo) => "That command cannot be undone now. No changes were made.",
        (Lang::PtBr, CannotUndo) => "Esse comando não pode ser desfeito agora. Nada foi alterado.",
        (Lang::Es, CannotUndo) => "Ese comando no puede deshacerse ahora. No se cambió nada.",
        (Lang::Zh, CannotUndo) => "现在无法撤销该命令。未进行任何更改。",
        (Lang::En, Unavailable) => {
            "That command is unavailable in the current snapshot. No changes were made."
        }
        (Lang::PtBr, Unavailable) => "Esse comando está indisponível no retrato atual. Nada foi alterado.",
        (Lang::Es, Unavailable) => {
            "Ese comando no está disponible con la información actual. No se cambió nada."
        }
        (Lang::Zh, Unavailable) => "当前快照无法执行该命令。未进行任何更改。",
        (Lang::En, Locked) => "Voice commands are unavailable while the screen is locked.",
        (Lang::PtBr, Locked) => "Comandos de voz ficam indisponíveis com a tela bloqueada.",
        (Lang::Es, Locked) => "Los comandos de voz no están disponibles con la pantalla bloqueada.",
        (Lang::Zh, Locked) => "锁屏时无法使用语音命令。",
        (Lang::En, NoWaiting) => "No session is waiting on a card.",
        (Lang::PtBr, NoWaiting) => "Nenhuma sessão está esperando uma resposta em um card.",
        (Lang::Es, NoWaiting) => "Ninguna sesión espera una respuesta en una tarjeta.",
        (Lang::Zh, NoWaiting) => "没有会话正在等待卡片回复。",
        (Lang::En, NoUsage) => "No plan usage is available in the local snapshot.",
        (Lang::PtBr, NoUsage) => "Não há uso do plano disponível no retrato local.",
        (Lang::Es, NoUsage) => "No hay datos de uso del plan en la información local.",
        (Lang::Zh, NoUsage) => "本地快照中没有套餐使用情况。",
        (Lang::En, NoPullRequests) => "The local GitHub snapshot has no open pull requests.",
        (Lang::PtBr, NoPullRequests) => "O retrato local do GitHub não tem pull requests abertos.",
        (Lang::Es, NoPullRequests) => "La información local de GitHub no tiene pull requests abiertos.",
        (Lang::Zh, NoPullRequests) => "本地 GitHub 快照中没有待处理的拉取请求。",
    }
}

pub fn voice_waiting(lang: Lang, agent: &str, project: &str) -> String {
    match lang {
        Lang::En => format!("{agent} in {project} is waiting for your answer on a card."),
        Lang::PtBr => format!("{agent} em {project} está esperando sua resposta em um card."),
        Lang::Es => format!("{agent} en {project} espera tu respuesta en una tarjeta."),
        Lang::Zh => format!("{project} 中的 {agent} 正在等待你回复卡片。"),
    }
}

pub fn voice_usage(lang: Lang, agent: &str, used: u64, minutes: u64) -> String {
    let left = 100u64.saturating_sub(used);
    match lang {
        Lang::En => format!("{agent}: {used}% used and {left}% remaining in the {minutes}-minute window."),
        Lang::PtBr => format!("{agent}: {used}% usado e {left}% restante na janela de {minutes} minutos."),
        Lang::Es => format!("{agent}: {used}% usado y {left}% restante en la ventana de {minutes} minutos."),
        Lang::Zh => format!("{agent}：{minutes} 分钟窗口内已用 {used}%，剩余 {left}%。"),
    }
}

pub fn voice_pull_request(
    lang: Lang,
    name: &str,
    title: &str,
    checks: Option<crate::board::Checks>,
) -> String {
    use crate::board::Checks::*;
    match (lang, checks) {
        (Lang::En, Some(Passing)) => format!("Pull request {name}: {title}. Checks are passing."),
        (Lang::PtBr, Some(Passing)) => format!("Pull request {name}: {title}. Os checks estão passando."),
        (Lang::Es, Some(Passing)) => format!("Pull request {name}: {title}. Los checks pasan."),
        (Lang::Zh, Some(Passing)) => format!("拉取请求 {name}：{title}。检查已通过。"),
        (Lang::En, Some(Failing)) => format!("Pull request {name}: {title}. Checks are failing."),
        (Lang::PtBr, Some(Failing)) => format!("Pull request {name}: {title}. Os checks estão falhando."),
        (Lang::Es, Some(Failing)) => format!("Pull request {name}: {title}. Los checks fallan."),
        (Lang::Zh, Some(Failing)) => format!("拉取请求 {name}：{title}。检查失败。"),
        (Lang::En, Some(Running)) => format!("Pull request {name}: {title}. Checks are running."),
        (Lang::PtBr, Some(Running)) => format!("Pull request {name}: {title}. Os checks estão em andamento."),
        (Lang::Es, Some(Running)) => format!("Pull request {name}: {title}. Los checks están en curso."),
        (Lang::Zh, Some(Running)) => format!("拉取请求 {name}：{title}。检查正在运行。"),
        (Lang::En, None) => format!("Pull request {name}: {title}. Check status is unavailable."),
        (Lang::PtBr, None) => format!("Pull request {name}: {title}. O estado dos checks está indisponível."),
        (Lang::Es, None) => {
            format!("Pull request {name}: {title}. El estado de los checks no está disponible.")
        }
        (Lang::Zh, None) => format!("拉取请求 {name}：{title}。检查状态不可用。"),
    }
}

pub fn voice_session(lang: Lang, agent: &str, project: &str, status: crate::Status) -> String {
    use crate::Status::*;
    match (lang, status) {
        (Lang::En, Idle) => format!("{agent} in {project} is idle."),
        (Lang::PtBr, Idle) => format!("{agent} em {project} está ocioso."),
        (Lang::Es, Idle) => format!("{agent} en {project} está inactivo."),
        (Lang::Zh, Idle) => format!("{project} 中的 {agent} 处于空闲状态。"),
        (Lang::En, Thinking) => format!("{agent} in {project} is thinking."),
        (Lang::PtBr, Thinking) => format!("{agent} em {project} está pensando."),
        (Lang::Es, Thinking) => format!("{agent} en {project} está pensando."),
        (Lang::Zh, Thinking) => format!("{project} 中的 {agent} 正在思考。"),
        (Lang::En, Working) => format!("{agent} in {project} is working."),
        (Lang::PtBr, Working) => format!("{agent} em {project} está trabalhando."),
        (Lang::Es, Working) => format!("{agent} en {project} está trabajando."),
        (Lang::Zh, Working) => format!("{project} 中的 {agent} 正在工作。"),
        (Lang::En, Approval) => format!("{agent} in {project} is waiting for permission."),
        (Lang::PtBr, Approval) => format!("{agent} em {project} está esperando uma permissão."),
        (Lang::Es, Approval) => format!("{agent} en {project} está esperando un permiso."),
        (Lang::Zh, Approval) => format!("{project} 中的 {agent} 正在等待权限批准。"),
        (Lang::En, Question) => format!("{agent} in {project} is waiting for an answer."),
        (Lang::PtBr, Question) => format!("{agent} em {project} está esperando uma resposta."),
        (Lang::Es, Question) => format!("{agent} en {project} está esperando una respuesta."),
        (Lang::Zh, Question) => format!("{project} 中的 {agent} 正在等待回答。"),
        (Lang::En, Finished) => format!("{agent} in {project} finished."),
        (Lang::PtBr, Finished) => format!("{agent} em {project} terminou."),
        (Lang::Es, Finished) => format!("{agent} en {project} terminó."),
        (Lang::Zh, Finished) => format!("{project} 中的 {agent} 已完成。"),
        (Lang::En, Failed) => format!("{agent} in {project} failed."),
        (Lang::PtBr, Failed) => format!("{agent} em {project} falhou."),
        (Lang::Es, Failed) => format!("{agent} en {project} falló."),
        (Lang::Zh, Failed) => format!("{project} 中的 {agent} 执行失败。"),
        (Lang::En, RateLimited) => format!("{agent} in {project} is rate limited."),
        (Lang::PtBr, RateLimited) => format!("{agent} em {project} está limitado pela cota."),
        (Lang::Es, RateLimited) => format!("{agent} en {project} está limitado por la cuota."),
        (Lang::Zh, RateLimited) => format!("{project} 中的 {agent} 已达到速率限制。"),
    }
}

/// Headings and source-local voice labels for text-only meeting exports.
#[derive(Debug)]
pub struct MeetingExport {
    pub microphone: &'static str,
    pub system: &'static str,
    pub summary: &'static str,
    pub points: &'static str,
    pub todos: &'static str,
    pub unknown: &'static str,
}

pub fn meeting_export(lang: Lang) -> MeetingExport {
    match lang {
        Lang::En => MeetingExport {
            microphone: "Microphone",
            system: "System audio",
            summary: "Summary",
            points: "Key points",
            todos: "To-dos",
            unknown: "Unknown voice",
        },
        Lang::PtBr => MeetingExport {
            microphone: "Microfone",
            system: "Áudio do sistema",
            summary: "Resumo",
            points: "Pontos principais",
            todos: "A fazer",
            unknown: "Voz desconhecida",
        },
        Lang::Es => MeetingExport {
            microphone: "Micrófono",
            system: "Audio del sistema",
            summary: "Resumen",
            points: "Puntos clave",
            todos: "Pendientes",
            unknown: "Voz desconocida",
        },
        Lang::Zh => MeetingExport {
            microphone: "麦克风",
            system: "系统音频",
            summary: "摘要",
            points: "要点",
            todos: "待办事项",
            unknown: "未知声音",
        },
    }
}

pub fn meeting_voice(lang: Lang, number: u32) -> String {
    match lang {
        Lang::En => format!("Voice {number}"),
        Lang::PtBr | Lang::Es => format!("Voz {number}"),
        Lang::Zh => format!("声音 {number}"),
    }
}
