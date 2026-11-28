import type { MeetingAssetsStatus } from "../../bridge";
import { el } from "../../dom";
import { t, tk } from "../../i18n";

export interface MeetingAssetsBackend {
  status(): Promise<MeetingAssetsStatus>;
  review(): Promise<MeetingAssetsStatus>;
  begin(id: string, consent: boolean): Promise<MeetingAssetsStatus>;
  cancel(id: string): Promise<MeetingAssetsStatus>;
}
const phases = {
  idle: tk("Review the optional package before downloading."),
  checking: tk("Checking installed meeting files…"),
  review: tk("Review ready. Confirm within one minute or review again."),
  downloading: tk("Installing meeting files…"),
  cancelling: tk("Cancelling and cleaning temporary meeting files…"),
  installed: tk("Meeting files verified. Installation does not enable automatic voice identification."),
  cancelled: tk("Installation cancelled. Previously installed files are preserved."),
  failed: tk("Meeting files could not be prepared."),
  unsupported: tk("This optional package requires Linux x86-64 with glibc."),
};
const artifacts = { segmentation: "pyannote segmentation-3.0", embedding: "CAM++", runtime: "ONNX Runtime 1.31.0" };
const running = (phase?: string) => ["checking", "downloading", "cancelling"].includes(phase ?? "");

export function createMeetingAssets(api: MeetingAssetsBackend) {
  const title = el("summary"), about = el("p", { class: "hint" }), contents = el("p", { class: "hint" });
  const policy = el("p", { class: "hint" }), phase = el("p"), feedback = el("p", { class: "setting-feedback error" });
  const progress = el("progress"), amount = el("p", { class: "hint" });
  const check = el("input"); check.type = "checkbox";
  const consentText = el("span");
  const consent = el("label", { class: "meeting-assets-consent" }, check, consentText);
  const review = el("button", { class: "btn" }), begin = el("button", { class: "btn primary" }), cancel = el("button", { class: "btn" });
  const actions = el("div", { class: "actions" }, review, begin, cancel);
  const packageTitle = el("summary");
  const packageDetails = el("details", { class: "meeting-assets-package" }, packageTitle, about, contents, policy);
  const element = el("details", { class: "card meeting-assets" }, title, phase, progress, amount, feedback, actions, consent, packageDetails);
  element.dataset.setting = "meeting-assets";
  packageDetails.dataset.setting = "meeting-assets-package";
  check.dataset.setting = "meeting-assets-consent";
  review.dataset.setting = "meeting-assets-review";
  begin.dataset.setting = "meeting-assets-begin";
  cancel.dataset.setting = "meeting-assets-cancel";
  phase.setAttribute("role", "status"); feedback.setAttribute("role", "alert");
  let value: MeetingAssetsStatus | null = null, visible = false, reading = false, busy = false, failed = false;
  let generation = 0, interval = 0;
  const active = () => visible && element.open && !document.hidden;
  const draw = () => {
    title.textContent = t("Meeting speaker models (experimental)");
    packageTitle.textContent = t("Package details and licenses");
    about.textContent = t("Optional local analysis files; installing them turns nothing on. Meeting quality is not validated yet.");
    contents.textContent = t("Download: {size} MB from GitHub release servers. Includes pyannote segmentation-3.0 (MIT), CAM++ (Apache-2.0) and ONNX Runtime 1.31.0 CPU (MIT), with licenses and third-party notices.", { size: ((value?.downloadBytes ?? 46647585) / 1e6).toFixed(1) });
    policy.textContent = t("Keeps installing if you leave this page; cancel here or quit the app to stop. Up to 24 requests (redirects included) in 24 hours, with no automatic retry or update check.");
    const label = value ? t(phases[value.phase]) : t("Reading installation status…");
    if (phase.textContent !== label) phase.textContent = label;
    const error = failed ? t("Could not update the status. Review again to reconnect; an installation may still be running.")
      : value?.error === "admission" ? t("Blocked by the request limit or local ledger. Review again later; restarting the app does not reset it.")
      : value?.error === "verification" ? t("Could not verify the files or local ledger; existing files were kept. Check local storage, then review again.")
      : value?.error ? t("Installation failed; existing versions were kept. Check the connection and storage, then review again.") : "";
    if (feedback.textContent !== error) feedback.textContent = error; feedback.hidden = !error;
    progress.hidden = value?.phase !== "downloading" || !value.total;
    progress.max = value?.total || 1; progress.value = value?.downloaded ?? 0;
    progress.setAttribute("aria-label", t("Meeting model download progress"));
    amount.hidden = progress.hidden;
    amount.textContent = t("{artifact}: {done} of {total} MB downloaded, then verified.", { artifact: artifacts[value?.artifact as keyof typeof artifacts] ?? "", done: ((value?.downloaded ?? 0) / 1e6).toFixed(1), total: ((value?.total ?? 0) / 1e6).toFixed(1) });
    consentText.textContent = t("I agree to download and install the optional package described in Package details and licenses.");
    consent.hidden = value?.phase !== "review" || failed;
    check.disabled = busy;
    review.textContent = t("Review installation"); begin.textContent = t("Download and install"); cancel.textContent = t("Cancel installation");
    review.hidden = running(value?.phase) || value?.phase === "unsupported";
    review.disabled = busy || reading;
    begin.hidden = consent.hidden; begin.disabled = busy || !check.checked;
    cancel.hidden = !running(value?.phase) && value?.phase !== "review";
    cancel.disabled = busy || value?.phase === "cancelling";
    element.setAttribute("aria-busy", String(busy || reading));
    const focused = document.activeElement;
    if ([begin, cancel, review].includes(focused as HTMLButtonElement) && (focused as HTMLElement).hidden) {
      const next = !cancel.hidden && !cancel.disabled ? cancel : !review.hidden ? review : title;
      next.focus({ preventScroll: true });
    }
  };
  const accept = (next: MeetingAssetsStatus) => {
    if (next.id !== value?.id || next.phase !== "review") check.checked = false;
    value = next; failed = false; draw();
  };
  const read = async () => {
    if (!active() || reading || busy) return;
    const current = generation; reading = true;
    try { const next = await api.status(); if (current === generation && active()) accept(next); }
    catch { if (current === generation && active()) { failed = true; check.checked = false; draw(); } }
    finally { reading = false; if (active()) draw(); if (current !== generation && active() && !busy) void read(); }
  };
  const act = async (operation: () => Promise<MeetingAssetsStatus>, reviewing = false) => {
    if (busy || !active()) return;
    busy = true; generation++; const current = generation; check.checked = false; draw();
    try {
      const next = await operation();
      if (current === generation && active()) accept(next);
      else if (reviewing && next.phase === "review") void api.cancel(next.id).catch(() => {});
    } catch { if (current === generation && active()) { failed = true; draw(); } }
    finally { busy = false; if (active()) draw(); }
  };
  check.addEventListener("change", draw);
  review.addEventListener("click", () => void act(() => api.review(), true));
  begin.addEventListener("click", () => { if (value?.phase === "review" && check.checked) { const id = value.id; value = { ...value, phase: "downloading" }; void act(() => api.begin(id, true)); } });
  cancel.addEventListener("click", () => { if (value) { const id = value.id; void act(() => api.cancel(id)); } });
  const visibility = () => {
    generation++; check.checked = false;
    if (interval) { window.clearInterval(interval); interval = 0; }
    if (active()) { void read(); interval = window.setInterval(() => { if (running(value?.phase)) void read(); }, 500); }
    else if (value?.phase === "review") { const id = value.id; value = null; void api.cancel(id).catch(() => {}); }
    draw();
  };
  element.addEventListener("toggle", visibility);
  draw();
  return { element, relabel: draw, visible(on: boolean) { if (visible === on) return; visible = on; visibility(); } };
}
