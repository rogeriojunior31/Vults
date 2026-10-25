import type { DictationDiagnostics, DictationMicTest } from "../../bridge";
import { el } from "../../dom";
import { t, tk } from "../../i18n";
import { button } from "./pieces";

type Backend = {
  report(): Promise<DictationDiagnostics>;
  start(): Promise<number>;
  status(id: number): Promise<DictationMicTest>;
  stop(id: number): Promise<void>;
};
const STAGES = [["total", tk("Release to delivery")], ["stt", tk("Transcription")],
  ["pipeline", tk("Text processing")], ["cleanup", tk("Model cleanup")], ["insert", tk("Delivery")]] as const;
const STATES: Record<string, string> = {
  ready: tk("Ready"), unavailable: tk("Unavailable"), off: tk("Off"), requesting: tk("Requesting…"),
  temporary: tk("Temporary access"), "not-needed": tk("Not needed for this session"), unverified: tk("Not verified"),
};
const VERDICTS = { ok: tk("Input level looks usable."), "no-signal": tk("No usable signal. Check your microphone and input volume."),
  noisy: tk("High background level. Try a quieter space or lower input volume."),
  clipping: tk("The input is clipping. Lower your microphone volume.") };

/** Kept across Settings repaints. Meter polling never replaces the focused action. */
export function createDictationDoctor(api: Backend) {
  const element = el("section", { class: "dictation-diagnostics" });
  const reportBody = el("div", { class: "diagnostic-report" });
  let countersExpanded = false;
  let reportRefresh: HTMLButtonElement;
  const micBody = el("div", { class: "card" });
  let visible = false, report: DictationDiagnostics | null = null, loading = false, failed = false, reportEpoch = 0;
  let id: number | null = null, starting = false, micEpoch = 0, mic: DictationMicTest | null = null, micFailed = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let bars: HTMLProgressElement[] = [], instruction: HTMLElement;
  const number = (value: number | null, suffix = "") => value === null ? t("Unknown") : `${value}${suffix}`;
  const fact = (name: string, value: string) => el("div", { class: "diagnostic-fact" }, el("dt", { text: name }), el("dd", { text: value }));
  const stop = () => {
    micEpoch++; starting = false;
    clearTimeout(timer);
    if (id !== null) void api.stop(id).catch(() => {});
    id = null; mic = null; micFailed = false;
    drawMic();
  };
  function updateMeter() {
    bars.forEach((bar,i) => { bar.value = mic?.bands[i] ?? 0; });
    instruction.textContent = starting ? t("Opening microphone…") : mic?.phase === "running"
      ? mic.elapsed_ms < 2000 ? t("Stay quiet for the noise measurement…") : t("Speak now to test your input…")
      : micFailed || mic?.phase === "error" ? t("Could not test the microphone. Check your input and try again.")
      : mic?.phase === "done" && mic.verdict ? t(VERDICTS[mic.verdict])
      : mic?.phase === "stopped" ? t("Microphone test stopped.") : t("The microphone is off.");
  }
  function drawMic() {
    const focused = micBody.contains(document.activeElement);
    const running = starting || id !== null;
    const start = button(t("Test microphone"), () => void startMic()) as HTMLButtonElement;
    start.dataset.setting = "diagnostic-mic-start";
    start.disabled = running;
    const cancel = button(t("Stop test"), stop) as HTMLButtonElement;
    cancel.dataset.setting = "diagnostic-mic-stop";
    cancel.disabled = !running;
    instruction = el("p", { class: "note" }); instruction.setAttribute("role", "status");
    bars = Array.from({ length: 9 }, () => {
      const bar = el("progress"); bar.max = 1; bar.value = 0; bar.setAttribute("aria-hidden", "true"); return bar;
    });
    const meter = el("div", { class: "diagnostic-bands" }, ...bars); meter.setAttribute("aria-hidden", "true");
    micBody.replaceChildren(el("h3", { text: t("Microphone test") }),
      el("p", { class: "hint", text: t("Six seconds: stay quiet for two, then speak. Audio stays in memory, is discarded after the test and is never transcribed.") }),
      el("div", { class: "actions" }, start, cancel), meter, instruction,
      ...(mic?.phase === "done" ? [el("p", { class: "hint", text: t("Estimated noise floor: {value} dBFS (digital level, not room loudness).", { value: number(mic.noise_db) }) })] : []));
    updateMeter();
    if (focused) (running ? cancel : start).focus({ preventScroll: true });
  }
  async function poll(epoch: number, test: number) {
    try {
      const next = await api.status(test);
      if (epoch !== micEpoch || !visible) return;
      mic = next;
      if (next.phase === "running") { updateMeter(); timer = setTimeout(() => void poll(epoch, test), 120); }
      else { id = null; drawMic(); }
    } catch {
      if (epoch !== micEpoch) return;
      stop(); micFailed = true; drawMic();
    }
  }
  async function startMic() {
    if (!visible || starting || id !== null) return;
    const epoch = ++micEpoch;
    starting = true; mic = null; micFailed = false; drawMic();
    try {
      const test = await api.start();
      if (epoch !== micEpoch || !visible) { void api.stop(test).catch(() => {}); return; }
      starting = false; id = test;
      mic = { id: test, phase: "running", elapsed_ms: 0, bands: [], noise_db: null, verdict: null };
      drawMic(); void poll(epoch, test);
    } catch {
      if (epoch !== micEpoch) return;
      starting = false; micFailed = true; drawMic();
    }
  }
  function drawReport() {
    const refresh = button(t("Refresh diagnostics"), () => void load()) as HTMLButtonElement;
    reportRefresh = refresh;
    refresh.dataset.setting = "diagnostic-refresh";
    refresh.disabled = loading;
    const info = el("p", { class: `note${failed ? " error" : ""}`, text: loading ? t("Reading dictation diagnostics…")
      : failed ? report ? t("Could not refresh diagnostics. The last readings may be out of date.") : t("Could not read diagnostics. Try again.") : "" });
    info.setAttribute("role", failed ? "alert" : "status"); info.hidden = !info.textContent;
    const children: HTMLElement[] = [el("div", { class: "actions" }, refresh), info];
    if (report) {
      const s = report.system;
      const state = (key: string) => STATES[key] ? t(STATES[key]) : t("Unknown");
      const yes = (value: boolean) => value ? t("Ready") : t("Unavailable");
      const system = el("section", { class: "card diagnostic-system" }, el("h3", { text: t("System") }), el("dl", { class: "diagnostic-facts" },
        fact(t("Audio host"), s.host ?? t("Unavailable")), fact(t("Active input"), s.device ?? t("Unavailable")),
        fact(t("Native input rate"), number(s.rate, " Hz")), fact(t("Desktop session"), s.session === "unknown" ? t("Unknown") : s.session),
        fact(t("GlobalShortcuts portal"), state(s.shortcuts)), fact(t("RemoteDesktop keyboard access"), state(s.keyboard)),
        fact(t("KWin focus script"), state(s.kwin)), fact(t("GPU transcription"), yes(s.gpu)), fact(t("Local VAD model"), yes(s.vad))),
        el("p", { class: "hint", text: t("Checks read current state without requesting permissions. Missing input uses the same fallback as dictation. GPU readiness does not measure speed.") }));
      const counters = el("details", { class: "diagnostic-counters" }, el("summary", { text: t("Counters since app start") }), el("dl", { class: "diagnostic-facts" },
        fact(t("Repetitive runs cut"), String(report.counters.loops)), fact(t("Base retries"), String(report.counters.retries)), fact(t("Copied outcomes"), String(report.counters.copied))),
        el("p", { class: "hint", text: t("Final dictation decodes only. Preview and Chat are excluded; Copy again can add a Copied outcome. Counters reset when Vults exits.") }));
      counters.dataset.setting = "diagnostic-counters"; counters.open = countersExpanded;
      counters.addEventListener("toggle", () => { countersExpanded = counters.open; });
      const timings = el("section", { class: "card diagnostic-timing-card" }, el("h3", { text: t("Stage timings") }),
        el("p", { class: "hint", text: t("Latest 50 retained, delivered dictations. Missing or unused stages stay unknown; erasing text erases its timings.") }));
      if (report.timings) {
        const table = el("table", { class: "diagnostic-timings" }, el("caption", { text: t("Measured durations in milliseconds") }),
          el("thead", {}, el("tr", {}, ...[t("Stage"), t("Samples"), "p50", "p95"].map(text => { const th = el("th", { text }); th.scope = "col"; return th; }))),
          el("tbody", {}, ...STAGES.map(([key,label]) => {
            const d = report!.timings![key]; const th = el("th", { text: t(label) }); th.scope = "row";
            return el("tr", {}, th, ...[String(d.count), number(d.p50_ms), number(d.p95_ms)].map(text => el("td", { text })));
          })));
        const scroll = el("div", { class: "diagnostic-table-scroll" }, table); scroll.tabIndex = 0;
        scroll.setAttribute("role", "region"); scroll.setAttribute("aria-label", t("Stage timings")); timings.append(scroll);
      } else timings.append(el("p", { class: "note error", text: t("Could not read timing history. Refresh diagnostics to try again.") }));
      children.push(timings, system, counters);
    }
    reportBody.replaceChildren(...children);
  }
  async function load() {
    if (loading || !visible) return;
    const origin = document.activeElement;
    const focused = reportBody.contains(origin);
    const epoch = ++reportEpoch; loading = true; failed = false; drawReport();
    try { const next = await api.report(); if (epoch === reportEpoch) report = next; }
    catch { if (epoch === reportEpoch) failed = true; }
    finally { if (epoch === reportEpoch) { loading = false; drawReport(); if (visible && focused && (document.activeElement === origin || document.activeElement === document.body)) reportRefresh.focus({ preventScroll: true }); } }
  }
  function relabel() {
    drawMic(); drawReport();
    element.replaceChildren(micBody, reportBody);
  }
  relabel();
  return { element, relabel, visible(on: boolean) {
    if (visible === on) return;
    visible = on;
    if (on) void load(); else { reportEpoch++; loading = false; stop(); }
  } };
}
