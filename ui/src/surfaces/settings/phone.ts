// Settings → Phone (plan-zeca L2): answering cards from a paired phone, how it reaches this
// computer, pairing by QR, and the phones paired.
import { Bridge, type Pairing, type PhoneEvent, type PhoneSettings } from "../../bridge";
import { el } from "../../dom";
import { locale, t } from "../../i18n";
import { button, row, toggle } from "./pieces";

let phone: PhoneSettings | null = null;
let message: { text: string; error: boolean } | null = null;
/** The QR on screen, and when it stops pairing. */
let pairing: (Pairing & { until: number }) | null = null;
let countdown = 0;
let rerender: () => void = () => {};

/** Reads the phone's settings again; `render` draws the page when it is the one open. */
export async function refreshPhone(render: () => void): Promise<void> {
  rerender = render;
  try {
    phone = await Bridge.phoneSettings();
  } catch {
    phone = null;
  }
  rerender();
}

/** How a pairing ended, from the app. */
export function phoneEvent(e: PhoneEvent): void {
  stopPairing();
  if (e.paired) message = { text: t("Paired with {name}.", { name: e.paired }), error: false };
  else if (e.expired) message = { text: t("The code expired. Pair again for a new one."), error: true };
  else if (e.error === "closed") message = { text: t("The phone is off, or today's budget is spent."), error: true };
  else message = { text: t("The phone could not be saved. Try again."), error: true };
  void refreshPhone(rerender);
}

function stopPairing(): void {
  pairing = null;
  window.clearInterval(countdown);
}

function said(text: string, error = false): void {
  message = { text, error };
  void refreshPhone(rerender);
}

export function phonePage(): HTMLElement[] {
  const head = [
    el("h1", { text: t("Phone") }),
    el("p", {
      class: "lede",
      text: t("Answer cards from your phone while you are away from the desk. The phone only decides; this computer does what you chose. Approving needs your fingerprint or face on the phone; denying does not."),
    }),
  ];
  if (!phone) return head;
  const p = phone;
  const rows: HTMLElement[] = [
    row(
      t("Answer cards from a phone"),
      t("Sends each card to your paired phones and reads their answers, through the relay you choose. Each request to the relay is on your behalf: at most 10,000 a day, each kept in the local trace. The relay sees only sealed messages, never a command or a project."),
      toggle(p.on, async (on) => {
        await Bridge.setPhone(on);
        p.on = on;
        if (!on) stopPairing();
        rerender();
      }),
    ),
  ];
  const transport = document.createElement("select");
  transport.className = "field select";
  transport.append(new Option(t("Through a relay"), "relay", false, p.transport === "relay"));
  transport.append(new Option(t("Straight (LAN or tailnet)"), "direct", false, p.transport === "direct"));
  transport.addEventListener("change", () => {
    message = null;
    Bridge.setPhoneTransport(transport.value as PhoneSettings["transport"])
      .then(() => refreshPhone(rerender))
      .catch((e) => said(String(e), true));
  });
  rows.push(row(t("How the phone reaches this computer"), t("Through a relay on the internet, or straight over your network or tailnet."), transport));
  const direct = p.transport === "direct";
  const input = document.createElement("input");
  input.className = "field";
  input.spellcheck = false;
  input.placeholder = direct ? "100.64.0.2:8787" : "https://relay.example.com";
  input.value = (direct ? p.listen : p.relay) ?? "";
  const save = () =>
    (direct ? Bridge.setPhoneListen(input.value) : Bridge.setPhoneRelay(input.value))
      .then(() => said(t("Saved.")))
      .catch((e) => said(String(e), true));
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") void save();
  });
  rows.push(
    row(
      direct ? t("Listen on") : t("Relay"),
      direct
        ? t("This computer's address on your network or tailnet, and a port. The phone must reach it.")
        : t("Its address. You can run your own (see The phone's relay in the docs); plain http only on a network you trust."),
      el("div", { class: "inline" }, input, button(t("Save"), () => void save(), true)),
    ),
  );
  const ready = p.on && (direct ? p.listen : p.relay);
  rows.push(
    row(
      t("Pair a phone"),
      t("Shows a code to scan with the phone, good for two minutes."),
      button(
        t("Pair"),
        () => {
          if (!ready) {
            said(t("Turn the phone on and set how it reaches this computer first."), true);
            return;
          }
          message = null;
          Bridge.phonePair()
            .then((qr) => {
              pairing = { ...qr, until: Date.now() + qr.seconds * 1000 };
              window.clearInterval(countdown);
              countdown = window.setInterval(() => {
                if (!pairing || Date.now() >= pairing.until) stopPairing();
                rerender();
              }, 1000);
              rerender();
            })
            .catch((e) => said(String(e), true));
        },
        true,
      ),
    ),
  );
  if (pairing) rows.push(qrRow(pairing));
  if (message) rows.push(el("div", { class: "row" }, el("p", { class: `note${message.error ? " error" : " ok"}`, text: message.text })));

  const devices = p.devices.length
    ? p.devices.map((d) =>
        row(
          d.name,
          d.revoked
            ? t("Revoked")
            : t("Paired on {date}", { date: new Date(d.pairedAt * 1000).toLocaleDateString(locale(), { dateStyle: "medium" }) }),
          d.revoked
            ? el("span")
            : button(t("Revoke"), () => {
                Bridge.phoneRevoke(d.id)
                  .then(() => said(t("Revoked: nothing it signs is taken now.")))
                  .catch((e) => said(String(e), true));
              }),
        ),
      )
    : [el("div", { class: "row" }, el("p", { class: "note", text: t("No phone paired yet.") }))];

  return [
    ...head,
    el("section", { class: "card rows" }, ...rows),
    el("h2", { text: t("Paired phones") }),
    el("section", { class: "card rows" }, ...devices),
    el(
      "section",
      { class: "card" },
      el("p", {
        class: "note",
        text: t("The Vults phone app is not out yet. Until it is, the command-line test client in the docs pairs and answers like a phone."),
      }),
    ),
  ];
}

/** The QR, drawn from its modules on white with the quiet margin a scanner needs. */
function qrRow(qr: Pairing & { until: number }): HTMLElement {
  const scale = 5;
  const margin = 4;
  const side = (qr.size + margin * 2) * scale;
  const canvas = document.createElement("canvas");
  canvas.width = side;
  canvas.height = side;
  canvas.className = "qr";
  const g = canvas.getContext("2d");
  if (g) {
    g.fillStyle = "#fff";
    g.fillRect(0, 0, side, side);
    g.fillStyle = "#000";
    for (let i = 0; i < qr.cells.length; i++) {
      if (qr.cells[i] !== "1") continue;
      g.fillRect((margin + (i % qr.size)) * scale, (margin + Math.floor(i / qr.size)) * scale, scale, scale);
    }
  }
  const left = Math.max(0, Math.ceil((qr.until - Date.now()) / 1000));
  const text = document.createElement("input");
  text.className = "field";
  text.readOnly = true;
  text.value = qr.text;
  text.addEventListener("focus", () => text.select());
  return el(
    "div",
    { class: "row qr-row" },
    canvas,
    el(
      "div",
      { class: "row-text" },
      el("div", { class: "row-title", text: t("Scan this with the phone within {s} s.", { s: String(left) }) }),
      el("div", { class: "row-about", text: t("The code's text, for the command-line test client:") }),
      text,
      button(t("Cancel"), () => {
        stopPairing();
        void Bridge.phonePairCancel();
        rerender();
      }),
    ),
  );
}
