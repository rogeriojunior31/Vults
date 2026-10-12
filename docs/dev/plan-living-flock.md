# Plan: the Living Flock UI

Internal board (2026-10-11). Brings the Living Flock prototype (design exploration 08, revision of
2026-10-11) into the app, on top of what wave 4 built. The prototype was drawn against `ac0b496`
(#237), before W1 to W11; this board checks it against `4bed7e7` (#266) and says where each new
feature lives in the new layout.

The prototype is the design (look, sizes, tabs, habitat). This board adds only what the app already
does, and changes the prototype where it conflicts with a rule, an ADR or `road-to-0.2.md`'s D1 to
D8. Step ids are `U`; they never reuse another board's letter.

## What the prototype decides

- **Four sizes of attention**, one identity:
  - **Mini** (340 x 38): the pill. Only the flock, no text, ever. The birds fly inside it. Hovering
    a bird makes it look at you, and land if it was flying. A click opens that bird's session; a
    right click opens the quick menu. Someone needs you: an amber border that pulses.
  - **Quick** (440 px): see and resolve without opening everything. A quick view of every area,
    with the card in the same place as today.
  - **Panel** (720 to 1280 px, resizable): investigate and organize.
  - **Full** (1200 px and up): the same panel with room for details. Esc returns to the previous
    size.
  - The size switcher sits in the header. Changing size keeps the session, the filters and the
    connection chosen.
- **One shared sky.** All birds live in it in every size. Selecting a session changes the details
  below it; the other birds stay. The flight has its own space, and never crosses commands,
  messages or approval fields.
- **Motion that tells** (the "Movimentos que contam" section):
  - With no task, after a short rest, a bird takes off and circles on its own thermal. Glides and
    wing beats are never in sync.
  - New work brings it back to the wire, into the existing read, edit and run poses.
  - A permission, a question or a failure brings it back faster, with the sign beside the session.
  - When it is done, it celebrates with its species' gesture, then rises to wait.
  - Reduced motion keeps every bird on the perch. A canvas off screen does not draw.
- **The Nest has five tabs**, plus Settings behind the gear: **Flock · Tasks · Chat · Activity ·
  Connections**.
  - Projects is no longer a tab: it becomes Activity's *By project* block.
  - Memory lives in the Chat, as a side panel.
  - Records become sub-tabs of Activity: Week · History · Cost · Traces · Audit.
- **The footer**: Zeca with "Zeca on · optional", *Open terminal*, *Back to the island*.
- **Ctrl+K palette**:
  - In any size.
  - It jumps to sessions, and opens `/task`, `/delegate`, `/continue` and `/remember`.
  - A pending card only leads to the island; the palette never answers.
- **Roadmap slots**:
  - **The Mini pill:** future news shows only as body, color or motion: a bird that arrives when a task is dispatched, Zeca's mood (K7).
  - **The card:** one line for its policy and risk ("allowed by your rule" now, Zeca's risk line with K3).
  - **Returning:** the debrief takes the empty card's place in Quick (K2).
  - **Settings › Agents:** becomes the capability table.

## Reconciled with what was built

The prototype predates wave 4. Where it and the code disagree, the code's feature stays and moves
into the prototype's layout:

| Built | Today | In the new layout |
|---|---|---|
| W1 capabilities (doctor) | Nest › Agents | Settings › Agents, under the installers (the prototype's own slot) |
| W2 policies | Settings › Approvals › Policies | Same place; the Quick card gains its one line: "Allowed by your policy" / "your rule" in Audit, the deny-first rule untouched |
| W3 tasks, `/task` | Nest › Tasks; the chat | Nest › Tasks; Quick › Tasks (a short list); the palette |
| W4/W4b/W10 start (Claude, Codex, the ACP agents) | Task buttons | The same buttons on the task card; a dispatched task's bird takes off from Zeca to the wire |
| W5 Start fresh / Go on / Hand off | Task buttons | The same, on the task card |
| W6 review loop | Task buttons and rounds | Rounds drawn on the task card |
| W7 `/continue` | The chat | The chat and the palette |
| W8 router, `/delegate` | The chat; the choice on the card | The same, plus the palette; the card keeps "the choice and why" |
| W9 spend and budget | Tasks' spend line and budget form; Usage | Activity › Cost (bars, budget tick, budget form, per roost); the 80 % warning is a notification as today |
| W11 History, Traces, Audit | Nest tabs | Activity › History, Traces, Audit; Audit shows who decided (you, a rule, a policy, the phone) |
| W11 Usage | Nest tab | Activity › Cost; the 5 h / 7 d meters stay in the header |
| M3/M9 roosts, shared memory | Nest › Roosts | Activity › By project (worktrees in rows: branch, agent, state, week time, search); PRs and CI in Connections › GitHub; *Share memory* on the roost's row |
| M1/M5/M6 memory | Nest › Memory | Chat's side panel: USER, MEMORY, the current roost; proposals as a quiet card (accept is a commit, undo a revert) |
| Week recap, year grid | Settings › Activity | Activity › Week; Settings › Activity keeps the switches (keep history, clear, save as image) |
| GitHub boards | Island tabs, one per connector | Connections (the hub) in Quick and in the Nest; *Add a connection* lists only connectors that exist (ADR 0011: no Linear or Slack until they work) |
| Island tabs Flock / Chat / Drop | Island header | Quick's tabs; *Drop a file* stays a target on the island, not a tab |
| Pill text (name, status, song, badges) | Mini | Gone from Mini (zero text). The song and "N new" move to Quick's header; the status reads from the bird and the amber border; the pill keeps an `aria-label` |
| Settings window | Its own window, 10 pages | The Nest's gear page; `open_settings(section)` opens the Nest there. One webview fewer (D6) |

Facts the prototype got wrong at `ac0b496`, now true:

- **Branch and worktree grouping needs no new metadata.** `app/src/roosts.rs` reads every worktree's branch and groups by repository. Only `SessionView` lacks `branch` and `roost`: U4 adds them.
- **The Nest is not only Chat.** It has nine tabs, which become the five above.
- **"Local model"** in the provider switch is the API chat pointed at a local server. It shows as *API*, as today, offered only when usable.
- **New agents** (OpenCode, Qwen, Copilot) have colors in `themes.css`. Each bird keeps its agent's color and its project's species.

## Rules this board keeps

- **D1: cards live on the island only.**
  - In Panel and Full, a session that needs you shows *Review on the island →* and never Allow/Deny.
  - The Chat's permission does the same, as today with `chatToIsland`.
  - The palette never answers.
- **D6: one live webview at rest.** Panel and Full are the Nest window: built on open, gone on close. Quick is the island, which fits the fixed 720 x 560 layer surface.
  - Full is the Nest maximized or in full screen. It never grows the layer-shell overlay, as the prototype says itself.
- **ADR 0010 / D7: Zeca is optional.**
  - With him off, the Nest still opens: Flock, Tasks, Activity, Connections and Settings.
  - Chat and *Ask Zeca* are gone, and the footer says "Zeca off".
  - This lifts today's `open_nest` early return (`app/src/lib.rs`).
- **ADR 0011: only actions that work.** Every button in the session details, the menu and the hub is one that works today.
- **D2 / D4.** The selection is core's `focus`, through an `Intent`.
  - The size, tab, filter and chosen connection are look, not meaning. The app keeps them as a small `Place` and hands it to the Nest on open, like `open_settings(section)`.

## Steps

One PR per step, each with its lab states, visual baselines (`-u` on purpose), the three catalogs
and the docs (`guide/` and `reference/`, EN and pt-BR) in the same PR.

| # | Step | Needs | Done when |
|---|---|---|---|
| U0 | **Done.** **Decision and reference.** ADR 0021: four sizes (Mini, Quick, Panel = Nest, Full = Nest in full screen), five Nest tabs plus the gear, Settings inside the Nest, Nest available with Zeca off. Commit the 08 prototype to `design/` as the reference | | ADR accepted |
| U1 | **Done.** **The Nest's shell.** Five tabs and the gear; the footer; the current content moved into its new home with no new look yet (Agents into Settings, records into Activity's sub-tabs, Roosts into *By project*, Memory into Chat's side panel, the week's recap into *Week*); the Nest opens from the tray and without Zeca, at a tab `open_nest` names. The gear opens the Settings window until U12 folds it in; Full and its Esc come with U2 | U0 | Every feature of the nine tabs reachable in the new layout; with Zeca off the Nest opens without Chat |
| U2 | **Done.** **Sizes and the switcher.** The switch says *Room* where this board says Panel: the tray already has a *Panel* presence. Mini, Quick, Panel and Full in the header; Ctrl+Alt+1 to 3 (global, through `platform::shortcuts`), Alt+1 to 5 for tabs inside a surface; Quick to Panel keeps the tab, and the session in front is core's focus in every size (the Nest's Flock shows it with U7; filters come with U7, the chosen connection with U11) | U1 | A visual test per size; a test that Room opens on what Quick showed |
| U3 | **Done.** **Mini, the pill without text.** 340 x 38; the birds fly inside it; hover looks and lands; click opens the bird's session in Quick; right click opens the quick menu; amber pulse when someone needs you; song and "N new" go to Quick's header | U2 | Visual tests for rest, flight, needs-you; no text node in the pill; screen reader label kept |
| U4 | **Done.** **`SessionView` gains `roost` and `branch`.** The app reads them with `roosts::repo_of` when a session is first seen and sends core an input; the TS mirror regenerates | | A `core` test; `SessionView` carries them for U7 and U10 to group by |
| U5 | **The shared sky's motion.** Idle birds rest, then soar on their own thermals; work brings them back to the wire; needs-you brings them back faster; done celebrates, then rises; a dispatched task's bird takes off from Zeca; reduced motion keeps all on the perch; off-screen canvases stop. Reuses `sky.ts`, `Bird.soar` and the poses; joins the flock board's PR4 (flight) instead of racing it | U3 | Clip tests per behavior with time frozen; the perf budget of `scripts/` unchanged |
| U6 | **Done** (#273 and its follow-up), except the debrief slot, which K2 fills. **Quick, the island at 440.** The habitat strip and the selected session's card; the *Needs you* filter; the folder next to the decision; the card's one policy line; quick views of Tasks (short list, *Open in the Nest*), Activity (this week) and Connections (GitHub groups); a reserved, empty debrief slot for K2 | U2, U4 | Card positions and Y/N, 1 to 9, Ctrl+Alt+Y/N unchanged (existing tests pass); a visual test per tab |
| U7 | **Done**, with today's motion (U5 brings the new one). **Nest › Flock.** The large habitat; *Find a session…* and the state filters; arrows move the selection; session details (Needs you → *Review on the island*, Changes +N −M, Activity steps, Context, *Ask Zeca about this session* when Zeca is on, plan usage); Full opens the details pane beside it | U1, U4, U5 | Visual tests in Panel and Full; no Allow/Deny button outside the island (a test) |
| U8 | **Done.** **Nest › Tasks.** Grouped by roost; the card shows goal, criteria, owner, status, cost, review rounds and the router's choice and why; Start, Start with another agent, Start fresh, Go on, Hand off, Review, Stop and the worktree as today | U1, U4 | Every action of today's Tasks tab still works (the existing tests); visual tests |
| U9 | **Done**: Recent and New beside the chat; History, Context, Claude / Codex / API, voice and files are the chat's own, as before. **Nest › Chat.** Conversations (*Recent*, *New*), reading area apart from the composer, History and Context panels (folder, model), Memory side panel with proposals; Claude / Codex / API; voice and attach as today | U1 | Memory accept and undo work from the panel; visual tests |
| U10 | **Done** (most of it came with U1; the budget form moved here last). **Nest › Activity.** Week (recap, bars, year grid, *By project* with share memory, save as image), History, Cost (windows, days, budget tick and form, per roost), Traces, Audit (who decided) | U1, U4 | Today's records tests pass on the new tabs; visual tests |
| U11 | **Done** for the Nest; the island keeps one tab per connector while GitHub is the only one (a single Connections tab comes with a second connector). **Connections.** The hub in Quick and the Nest; GitHub with All / Needs review / Checks failed, your PRs, reviews waiting, default branches; a roost's PR and CI link from *By project*; the island's board tabs removed; *Add a connection* lists only real connectors | U1, U6 | The GitHub card's tests on the hub; nothing offered that does not work |
| U12 | **Settings in the gear.** The ten pages regrouped with the prototype's titles (General with the shortcuts table, Agents with the doctor, Chat & voice, Approvals, Projects, Activity, Connectors, Phone, Flock & Zeca, About); `settings.html` removed | U1 | Every setting of today still reachable; `open_settings(section)` lands on each page |
| U13 | **The palette (K9, pulled in).** Ctrl+K in Quick (keys mode) and in the Nest: sessions, `/task`, `/delegate`, `/continue`, `/remember`, open a tab; a session that waits leads to the island | U6, U7 | A test: the palette never answers a card (D1) |

U4 can start right away, in parallel with U0 to U2. U5 waits for the flock board's PR3 (doze, wake,
courtship), which is waiting for the user's ok.

## Order and size

About five to six weeks for one person with coding agents:

1. **Week 1:** U0, U1, U4.
2. **Week 2:** U2, U3.
3. **Weeks 3–4:** U5, U6, U7.
4. **Weeks 5–6:** U8 to U13.

Most island baselines change in U3 and U6. Each step refreshes only its own baselines.

## Decisions

Settled on 2026-10-11:

1. **The new UI is 0.2.0's UI.** The R3 smoke runs after U1, on the layout, and again after U13. The 0.2.0 tag comes after that second smoke.
2. **Tasks stay with Zeca off.** A task is the user's own work. Only `/task` and `/delegate` in the chat go with him.
3. **Settings lives inside the Nest** (U12). That means one webview fewer.
