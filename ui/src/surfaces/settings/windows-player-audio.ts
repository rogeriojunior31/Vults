import { type AudioChoice } from "../../bridge";
import { el } from "../../dom";
import { t } from "../../i18n";
import { button, row } from "./pieces";

type Native = { status(): Promise<AudioChoice | null>; list(): Promise<AudioChoice[]>; select(token: string | null): Promise<void> };
export function createWindowsPlayerAudio(native: Native, repaint: () => void) {
  let active = false, enabled = false, revision = 0, busy = false;
  let choices: AudioChoice[] = [], selected = "", message = "";
  function reset() { revision++; busy = false; choices = []; selected = ""; message = ""; }
  return {
    context(visible: boolean, allowed: boolean) {
      const entering = visible && allowed && (!active || !enabled);
      if (enabled !== allowed || active && !visible && busy) {
        reset();
        if (active) void native.select(null).catch(() => {});
      }
      active = visible; enabled = allowed;
      if (entering && !busy) {
        const own = revision;
        void native.status().then(choice => {
          if (own !== revision || !active || !enabled || !choice) return;
          choices = [choice]; selected = choice.token;
          message = t("Audio app linked for this session."); repaint();
        }, () => {});
      }
    },
    changed() { reset(); if (active) repaint(); },
    render() {
      const refresh = button(t("Refresh audio apps"), async () => {
        if (!active || !enabled || busy) return;
        const own = ++revision;
        busy = true; choices = []; selected = ""; message = ""; repaint();
        try {
          const result = await native.list();
          if (revision !== own || !active || !enabled) return;
          choices = result;
          message = result.length ? t("Choose the app that plays the selected media.") : t("No application audio sessions found. Start playback and refresh.");
        } catch {
          if (revision === own && active) message = t("Could not read audio apps. Check the media grants and refresh.");
        } finally { if (revision === own) { busy = false; repaint(); } }
      });
      refresh.disabled = !enabled || busy;
      const select = el("select", { class: "field" }) as HTMLSelectElement;
      const empty = el("option", { text: t("No audio app selected") }) as HTMLOptionElement; empty.value = ""; select.append(empty);
      for (const choice of choices) { const option = el("option", { text: t("{app} · {output} · process {pid}", { app: choice.app, output: choice.output ?? t("Audio output"), pid: choice.pid }) }) as HTMLOptionElement; option.value = choice.token; select.append(option); }
      select.value = selected; select.disabled = !enabled || busy || !choices.length;
      select.addEventListener("change", () => {
        const token = select.value;
        if (!active || !enabled || busy) return;
        const own = ++revision; busy = true; selected = ""; repaint();
        void native.select(token || null).then(() => {
          if (revision === own && active && enabled) { selected = token; message = token ? t("Audio app linked for this session.") : t("No audio app selected"); }
        }, () => {
          if (revision === own && active) message = t("Audio app changed. Refresh and choose again.");
        }).finally(() => { if (revision === own) { busy = false; repaint(); } });
      });
      const status = el("p", { class: "hint", text: busy ? t("Saving…") : message }); status.setAttribute("role", "status");
      const section = el("section", { class: "card rows" },
        row(t("Player audio app"), t("Temporarily link the selected player to an app in the Windows mixer. Applies to all audio in that app group, including browser tabs. Refresh clears the link. Choose again after the app or output changes."), refresh),
        row(t("Audio app"), t("Enable voice commands, Media voice commands and Now playing, then refresh while media is playing."), select),
        status,
      );
      section.dataset.setting = "windows-player-audio";
      return section;
    },
  };
}
