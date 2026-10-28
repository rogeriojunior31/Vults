import type { VoiceRoutes, VoiceRouteSelection } from "../../../bridge";
import { el } from "../../../dom";
import { t } from "../../../i18n";

export function createVoiceRoute(load: (() => Promise<VoiceRoutes>) | undefined, changed: () => void) {
  const mode = el("select"), source = el("select"), sink = el("select"), pair = el("select"), confirmed = el("input");
  mode.dataset.voiceRoute = "mode"; source.dataset.voiceRoute = "source"; sink.dataset.voiceRoute = "sink"; pair.dataset.voiceRoute = "pair"; confirmed.dataset.voiceRoute = "consent";
  confirmed.type = "checkbox"; confirmed.checked = false;
  const title = el("span"), microphone = el("span"), output = el("span"), echo = el("span"), consent = el("span");
  const refresh = el("button"), note = el("p", { class: "dictation-note" }), feedback = el("p", { class: "meeting-feedback" });
  feedback.setAttribute("role", "status");
  const inputs = el("div", { class: "voice-route-devices" }, el("label", {}, microphone, source), el("label", {}, output, sink));
  const echoes = el("label", {}, echo, pair), confirm = el("label", { class: "voice-conversation-automatic" }, confirmed, consent);
  const details = el("div", {}, refresh, inputs, echoes, note, confirm, feedback);
  const element = el("div", { class: "voice-route" }, el("label", {}, title, mode), details);
  for (const value of ["manual", "headphones", "echo"]) { const option = el("option"); option.value = value; mode.append(option); }
  mode.value = "manual";
  let inventory: VoiceRoutes | null = null, active = false, locked = false, pending = false, failed = false, epoch = 0;
  function fill(select: HTMLSelectElement, values: { name: string; label: string }[], placeholder: string) {
    const options = [el("option", { text: placeholder }), ...values.map(device => {
      const option = el("option", { text: device.label === device.name ? device.label : `${device.label} (${device.name})` }); option.value = device.name; return option;
    })];
    options[0].value = ""; select.replaceChildren(...options); select.value = "";
  }
  function selection(): VoiceRouteSelection | undefined {
    if (mode.value === "manual" || !inventory) return undefined;
    const chosen = mode.value === "echo" ? inventory.pairs[Number(pair.value)] : { source: source.value, sink: sink.value };
    if (mode.value === "echo" && pair.value === "" || !chosen?.source || !chosen.sink) return undefined;
    return { token: inventory.token, mode: mode.value as "headphones" | "echo", source: chosen.source, sink: chosen.sink, confirmed: confirmed.checked };
  }
  function draw() {
    title.textContent = t("Microphone mode");
    ["manual", "headphones", "echo"].forEach((value, i) => { mode.options[i].textContent = value === "manual" ? t("Manual turns") : value === "headphones" ? t("Continuous microphone with headphones") : t("Continuous microphone with echo cancellation"); });
    microphone.textContent = t("Microphone"); output.textContent = t("Audio output"); echo.textContent = t("Installed echo-cancellation pair");
    if (source.options[0]) source.options[0].textContent = t("Choose a microphone");
    if (sink.options[0]) sink.options[0].textContent = t("Choose an audio output");
    if (pair.options[0]) pair.options[0].textContent = t("Choose an installed pair");
    refresh.textContent = pending ? t("Loading audio devices…") : t("Refresh audio devices");
    details.hidden = mode.value === "manual";
    inputs.hidden = mode.value !== "headphones"; echoes.hidden = mode.value !== "echo";
    note.textContent = mode.value === "headphones" ? t("Use headphones to avoid feedback. Device names do not prove that you are wearing them.") : t("Only existing compatible AEC pairs are listed. Vults does not install modules or change system defaults.");
    consent.textContent = t("Keep this microphone open until I end the conversation, including while Zeca speaks. Confirmed speech may interrupt him and start a recording; Finish recording sends each turn.");
    feedback.textContent = failed ? t("Could not list audio devices. Refresh and choose the route again.") : inventory && mode.value === "echo" && !inventory.pairs.length ? t("No compatible echo-cancellation pair found. Configure one outside Vults, then refresh.") : "";
    feedback.hidden = !feedback.textContent;
    for (const control of [mode, source, sink, pair, confirmed, refresh]) control.disabled = !active || locked || pending;
    if (!load) { for (const option of [...mode.options].slice(1)) option.disabled = true; refresh.disabled = true; }
  }
  for (const control of [mode, source, sink, pair]) control.addEventListener("change", () => { if (locked || pending) return; confirmed.checked = false; draw(); changed(); if (control === mode && mode.value !== "manual" && !inventory) void reload(); });
  confirmed.addEventListener("change", () => { draw(); changed(); });
  async function reload() {
    if (!load || refresh.disabled) return;
    const mine = ++epoch; pending = true; failed = false; inventory = null; confirmed.checked = false; draw(); changed();
    try {
      const value = await load();
      if (mine !== epoch || !active) return;
      inventory = value;
      fill(source, value.sources, t("Choose a microphone")); fill(sink, value.sinks, t("Choose an audio output"));
      fill(pair, value.pairs.map((p, i) => ({ name: String(i), label: `${p.source} → ${p.sink}` })), t("Choose an installed pair"));
    } catch { if (mine === epoch) failed = true; }
    finally { if (mine === epoch) { pending = false; draw(); changed(); } }
  }
  refresh.onclick = reload;
  fill(source, [], t("Choose a microphone")); fill(sink, [], t("Choose an audio output")); fill(pair, [], t("Choose an installed pair")); draw();
  return {
    element, selection, summary: () => {
      if (mode.value === "manual") return t("Microphone and audio output follow the system defaults.");
      const chosen = selection();
      if (!chosen) return t("Choose a microphone and audio output before starting.");
      return t("Microphone: {microphone} · Output: {output}", { microphone: inventory?.sources.find(device => device.name === chosen.source)?.label ?? chosen.source, output: inventory?.sinks.find(device => device.name === chosen.sink)?.label ?? chosen.sink });
    }, manual: () => mode.value === "manual",
    ready: () => !pending && (mode.value === "manual" || !!selection()?.confirmed),
    clearConsent() { confirmed.checked = false; draw(); },
    expire() { inventory = null; confirmed.checked = false; failed = false; draw(); },
    update(on: boolean, busy: boolean) { active = on; locked = busy; draw(); },
    leave() { ++epoch; active = false; pending = false; confirmed.checked = false; draw(); },
  };
}
