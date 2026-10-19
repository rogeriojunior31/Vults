import { Bridge } from "../../bridge";
import { dictationStatus } from "../../island/dictation";
import { setLang } from "../../i18n";
import { applyTheme } from "../../theme";

const feedback = dictationStatus();
// Wait for lock/approval context before displaying even a cached recording status.
feedback.visible(false);
let haveView = false, haveSettings = false, paused = false, presenceChanged = false, languageChanged = false;
let ready = false, announcing = false;
const show = () => {
  feedback.visible(ready && haveView && haveSettings && !paused);
  if (!ready && !announcing && haveView && haveSettings) {
    announcing = true;
    void Bridge.dictationOverlayStarted().then(() => { ready = true; show(); }, () => { announcing = false; });
  }
};
Bridge.onView(view => {
  feedback.context(view);
  haveView = true;
  show();
});
void Bridge.appSettings().then(settings => {
  applyTheme(settings.theme, true);
  if (!languageChanged) setLang(settings.lang);
  haveSettings = true;
  feedback.captions(settings.dictationCaption, true);
  if (!presenceChanged) paused = settings.presence === "paused";
  show();
  feedback.relabel();
});
Bridge.onSettings(settings => {
  applyTheme(settings.theme);
  if (settings.presence !== undefined) { presenceChanged = true; paused = settings.presence === "paused"; show(); }
  if (settings.lang !== undefined) { languageChanged = true; setLang(settings.lang); feedback.relabel(); }
  if (settings.dictationCaption !== undefined) feedback.captions(settings.dictationCaption);
});
