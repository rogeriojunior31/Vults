# Local environment: functions, CLI mapping and Zeca integration

This is the technical companion to [design.md](design.md) and [index.html](index.html).
All native operations below are **proposed implementation**, not code installed by
this prototype. The page runs in memory: it never calls Docker, Podman, an HTTP
endpoint or a model. Clipboard and text download are real browser interactions.

Commands are human-readable examples. `CONTAINER`, `POD`, `CONTEXT`, `CONNECTION`
and `FILE` stand for validated targets. Native code must call the executable with
separate arguments, not pass these displayed strings to a shell. Resolve a Compose
file to an existing absolute path; `~` in the demo's displayed path is only an example.

## 1. Existing owners and the smallest integration

| Existing source | What should be reused | What this proposal adds |
| --- | --- | --- |
| [Connections renderer](../../ui/src/surfaces/nest/connections.ts) | Connector selection, status, filtering, rows, inspection, focus restoration and Ask Zeca entry | Local-service views and actions, preserving GitHub behavior |
| [Nest composition](../../ui/src/surfaces/nest/render.ts) | `createConnections` receives the Zeca-enabled callback; it opens Chat and calls `chat.draft(text)` | Prepare a local-service question at the same handoff |
| [Chat](../../ui/src/island/chat.ts) | `ChatPanel.draft` appends to existing composer text and focuses it | Retain the service capture/source when a local question is prepared |
| [Connector runtime](../../crates/connectors/src/lib.rs) and [app owner](../../app/src/connectors.rs) | Enabled state, snapshots, diffs, status, refresh and existing event routing | A local adapter with typed inspection and explicit action capabilities |
| [Connector registry](../../ui/src/connectors.ts) | Only offer implemented connectors; activation belongs to Settings | Register the local connector only when it exists |
| [Connector guide](../../docs/contributing/connectors.md) | Snapshot identity, baseline without alert floods, event keys/topics and backoff | Local state transitions with stable engine-qualified targets |
| [Open actions](../../app/src/open.rs) | Native argument handling and owned opening capabilities | A separately scoped terminal/URL handoff; do not invent a shell launcher in the renderer |

Current `Row`, `Group`, `Facts` and `StoryKind` contracts are GitHub-oriented.
Container states, resource samples and HTTP results do not already fit those
contracts. Native work must choose a small typed extension/projection at the
existing boundary; stuffing them into PR fields or duplicating the entire
Connections renderer is not the integration described here. No additional crate
or generic orchestration platform is required by this design.

The existing polling runtime is not already a lifecycle executor, a PTY supervisor
or a streaming log owner. Start with bounded snapshots; add owned streams only for
visible inspections. Reuse existing pause/lock/disabled semantics and existing
network-gate ownership. A remote engine request remains network activity even when
it is issued through a CLI. Do not silently change all connector budgets or policies.

## 2. Presentation functions: no independent CLI execution

These functions operate on captured state. They do not need another command each
time the user clicks or types.

| Function in the prototype | Data or native implementation needed | Zeca relationship |
| --- | --- | --- |
| Select GitHub / Local environment | Existing connector registry, enabled/status state | Use the selected source, keeping GitHub's current questions |
| Projects overview and totals | Normalized services, optional project IDs and runtime groups | Draft only after choosing a concrete service |
| Failures-first highlights | Sort a copy of captured services; show up to three | Do not invent an impact diagnosis from a count |
| Inspect a project | Apply that project filter, open Containers or Servers | A grouping is not a new agent execution scope |
| Problems | Unexpected failure facts and seen-state revision | Send evidence of the actual unresolved condition |
| Servers list and details | Configured HTTP checks and their response facts | HTTP data does not imply access to the server's source/logs |
| Containers list | All discovered containers, including stopped ones | Include selected engine/connection and immutable ID |
| Project / engine / text filters | Local filtering, intersecting all active filters | Changing a filter must not retarget an existing draft |
| Row selection and mobile Back | Selection/focus only; retain source identity | Ask uses the newly selected item, not an earlier item |
| Expand configuration / inspector tabs | Existing bounded capture; fetch extra data on demand | Include only fields relevant to the user's question |
| Search / clear logs | Text filter on the captured log lines | A search with zero results is not proof there were no other logs |
| Mark an alert seen | Store acknowledgement against service/state revision | Does not resolve a fault or change runtime state |
| Events and per-container history | Normalized engine changes plus separately recorded user operations | Keep engine observation and user action provenance distinct |
| Port inventory / open owning service | Published mappings and configured HTTP associations | A mapping alone is not a successful HTTP/TCP health test |
| Review diagnostic | Bounded snapshot of the selected project's services | User may choose to use that same evidence in a question |
| Copy / download diagnostic | Clipboard or user-selected text-file export | No implicit upload, model call or filesystem export |
| Alerts: changes / failures / silent | Existing notification routing and persisted preferences through the appropriate owner | Silence does not hide facts from an explicit diagnosis |
| Refresh | Request a scoped snapshot refresh, deduplicate in-flight work | A prepared draft remains a capture unless explicitly refreshed |
| Empty states / validation errors | Real availability and configured services, not fake success | No model call needed to explain an empty filter |
| Room / Full and See more | Preview layout, scroll and focus behavior | No change to conversation identity or agent authority |
| Scenarios / reset | Demonstration controls only | Never ship them as runtime controls |

Production attention/Seen behavior remains owned by the current application. The
local Problems list may dismiss its acknowledged alert, while Servers/Containers
retain the item and real failure totals. Do not change global Flock/Quick/Room
semantics to make this demonstration easier to reproduce.

## 3. Discovery, configuration and resource data

Docker and Podman both expose these operations, but their JSON shapes and formats
are different. Parse each engine's output with recorded fixtures and normalize
facts; never parse a localized, human-formatted table as an API.

| Function | Docker CLI example | Podman CLI example | Required treatment |
| --- | --- | --- | --- |
| Discover containers | `docker ps --all --no-trunc --format json` | `podman ps --all --no-trunc --format json` | Retain full IDs and stopped/paused states. Docker formatted JSON can be newline-delimited objects; Podman commonly returns an array. Do not assume the same shape. |
| Inspect configuration | `docker inspect CONTAINER` | `podman inspect CONTAINER` | Extract selected state, image, healthcheck configuration, mounts, networks and group facts; avoid automatically exporting all environment values. |
| CPU / RAM / resource snapshot | `docker stats --no-stream --format json CONTAINER` | `podman stats --no-stream --format json CONTAINER` | Preserve sample time, units and unavailable fields. A snapshot is not a time-series graph. |
| Published ports | `docker port CONTAINER` plus inspect | `podman port CONTAINER` plus pod inspect | Normalize mappings. Pod networking can publish at the pod level; avoid duplicating them for every member. |
| Mounted volumes / bind paths | Inspect `Mounts` | Container/pod inspection | Distinguish named volume, bind source, destination and access. Do not edit/delete mounts from a read-only inspector. |
| Network detail | `docker network inspect NETWORK` | `podman network inspect NETWORK` | Optional drilldown for a known selected network; not a scan of the machine. |
| Image detail | `docker image inspect IMAGE` | `podman image inspect IMAGE` | Optional image metadata; do not imply that checking an image pulls an update. |
| Volume detail | `docker volume inspect VOLUME` | `podman volume inspect VOLUME` | Optional named-volume metadata; a bind path is not a named volume. |
| Running processes | `docker top CONTAINER` | `podman top CONTAINER` | Query a running container. Container PIDs and host PIDs have different identities. |
| Files changed in writable layer | `docker diff CONTAINER` | `podman diff CONTAINER` | Added, changed and deleted paths. This is not version history or a diff of mounted-volume contents. |
| Engine events | `docker events --format json` | `podman events --format json` | Supervise a bounded/owned stream, normalize events and reconcile with snapshots after reconnect. |

Resource restrictions are capabilities, not errors to hide. In particular, Podman
rootless networking metrics may be unavailable. Show **Unavailable**, not zero.
Stopped containers do not get fabricated live resource readings; paused values
must identify their sample age. Short `--no-stream` CPU samples do not replace
sustained measurements. Restart counts and exit codes require engine-specific
inspection; they are not interchangeable with a history of user restart clicks.

References: [Docker container listing](https://docs.docker.com/reference/cli/docker/container/ls/),
[Podman listing](https://docs.podman.io/en/latest/markdown/podman-ps.1.html),
[Docker container commands](https://docs.docker.com/reference/cli/docker/container/),
[Podman commands](https://docs.podman.io/en/latest/Commands.html),
[Podman stats](https://docs.podman.io/en/latest/markdown/podman-stats.1.html),
[Docker events](https://docs.docker.com/reference/cli/docker/system/events/) and
[Podman events](https://docs.podman.io/en/latest/markdown/podman-events.1.html).

## 4. Logs, terminal and health

| Function | Docker | Podman | Effect and limitations |
| --- | --- | --- | --- |
| Recent logs | `docker logs --tail 30 CONTAINER` | `podman logs --tail 30 CONTAINER` | Bounded read of stdout/stderr retained by the engine; logging-driver availability matters. |
| Follow and time window | `docker logs --follow --timestamps --tail 100 --since 5m CONTAINER` | `podman logs --follow --timestamps --tail 100 --since 5m CONTAINER` | Owned stream for the selected inspection. Cancel/close stops the reader, not the container. |
| Terminal preview | `docker exec -it CONTAINER /bin/sh` | `podman exec -it CONTAINER /bin/sh` | Opening an interactive shell executes inside a running container; it is not a read-only inspection. The image may not include that shell. |
| Inspect Docker health | `docker inspect --format '{{json .State.Health}}' CONTAINER` | Use engine-specific inspection for saved health results | Docker command reads recorded healthcheck state; it does not execute a fresh check. |
| Run a configured check | No equivalent generic `docker healthcheck run` command in this flow | `podman healthcheck run CONTAINER` | Podman executes the configured command. Require configured healthcheck and running state; treat it as execution, not metadata lookup. |

The toolbox only copies command previews. The embedded Logs tab filters a fixed
sample. Neither starts an actual stream. Native work needs output bounds, explicit
process ownership, cancellation and an unavailable/error state. A terminal needs
an actual interactive terminal/PTY handoff, not execution in the background followed
by a fake success toast.

References: [Docker logs](https://docs.docker.com/reference/cli/docker/container/logs/),
[Podman logs](https://docs.podman.io/en/latest/markdown/podman-logs.1.html),
[Docker inspect](https://docs.docker.com/reference/cli/docker/container/inspect/) and
[Podman manual healthcheck](https://docs.podman.io/en/latest/markdown/podman-healthcheck-run.1.html).

## 5. Existing-container lifecycle

| Confirmed action | Docker | Podman | Meaning |
| --- | --- | --- | --- |
| Start | `docker start CONTAINER` | `podman start CONTAINER` | Start an existing stopped container; do not create or replace it. |
| Stop | `docker stop CONTAINER` | `podman stop CONTAINER` | End its process/connections; keep the container and volumes. |
| Restart | `docker restart CONTAINER` | `podman restart CONTAINER` | Restart its process; configuration or dependency failures can remain. |
| Pause | `docker pause CONTAINER` | `podman pause CONTAINER` | Suspend processes; do not claim that the application is serving. |
| Resume | `docker unpause CONTAINER` | `podman unpause CONTAINER` | Resume a paused process; independently confirm application health. |

In the preview these actions change sample state after confirmation. Redis's
permission failure and the worker's database failure survive restart/start in the
failure scenario. Cancel leaves every field and event unchanged.

Native sequence: resolve the exact connection/full ID, refresh and validate state,
show target and impact, accept the user's action through the existing application
boundary, supervise the command, record the outcome and refresh facts. Command
exit success is not proof of healthy application state. Do not operate on a
replacement container with the same name. Timeouts may leave an uncertain outcome;
re-read state rather than blindly retrying a mutation.

Remove, prune, kill, pull/rebuild, volume deletion and policy edits are outside the
implemented prototype. Existing destructive-action and human-permission rules
remain in force for any later expansion.

## 6. Compose and pod group operations

| Function | Docker Compose | Podman pod |
| --- | --- | --- |
| Discover groups and membership | `docker compose ls --format json`; `docker compose -f FILE -p PROJECT ps --all --format json`, confirmed against labels/configuration | `podman pod ps --format json`; `podman pod inspect POD` |
| Follow group logs | `docker compose -f FILE -p PROJECT logs --follow --tail 30` | `podman pod logs --follow --tail 30 POD` |
| Start existing members | `docker compose -f FILE -p PROJECT start` | `podman pod start POD` |
| Stop group | `docker compose -f FILE -p PROJECT stop` | `podman pod stop POD` |
| Restart group | `docker compose -f FILE -p PROJECT restart` | `podman pod restart POD` |
| Plan initialization | `docker compose -f FILE -p PROJECT --dry-run up -d` | No equivalent dry-run step is offered by this preview |

The shown Compose file is not discovered from this checkout. Production must
retain the actual config files, project directory, project name and selected
profiles; an inferred name is insufficient. A group's full affected membership
must be shown before an action, including members outside the current UI filter.
Pod infra containers belong to the pod even if application rows omit them.

A Vults project can contain independent HTTP servers and multiple engine groups.
Stopping one group does not stop those independent processes. Starting existing
containers is different from `compose up`, which may create/replace resources.
Dry-run is a planning capability, not a guarantee that a later execution has the
same inputs, dependencies or outcome. Group execution needs per-member results and
partial-failure handling; it is not implemented by copying a command.

References: [Compose commands and dry-run](https://docs.docker.com/reference/cli/docker/compose/),
[Compose start](https://docs.docker.com/reference/cli/docker/compose/start/),
[Podman pod commands](https://docs.podman.io/en/latest/markdown/podman-pod.1.html) and
[Podman pod logs](https://docs.podman.io/en/latest/markdown/podman-pod-logs.1.html).

## 7. HTTP servers and associated checks

No Docker/Podman operation is needed for an independent endpoint check. A native
HTTP client provides status, latency, timeout and connection errors. Reuse an
existing HTTP client instead of requiring curl to be installed. The displayed
`curl -i URL` or `curl -I URL` is a convenience command to copy; the latter uses
HEAD and is not equivalent to a GET health check.

| Function | Native requirement | What the prototype does |
| --- | --- | --- |
| Add server | Persist a reviewed monitoring configuration under the existing configuration owner | Validate name/local URL and add an illustrative HTTP result |
| Optional project | Store a nullable project reference; do not infer filesystem authority from a label | Group unassigned checks under Without project |
| Associate with container | Keep one service ID and a separate HTTP result owned by that service | Attach an unverified endpoint without increasing service count |
| Automatic checks | Enabled feature, explicit method/expected response/timeout/interval, bounded response handling | No scheduler or requests; initial timings are examples |
| Refresh | Request a real check and retain failure/stale status when appropriate | Redraw illustrative state only |
| Open endpoint | Native URL opener with validated local target | Show a simulated-open message |
| Inspect port conflict | Separate verified process/port evidence if explicitly provided | Display the API's sample EADDRINUSE log, without claiming to know the port owner |

The local-address restriction must be enforced natively as well as in the form,
including redirect targets and resolved addresses. A URL must be one argument
when rendered as a shell copy command. Process logs/PIDs require a linked source;
HTTP monitoring cannot manufacture them. A container's HTTP check, image-defined
healthcheck and process state remain separate fields.

## 8. Connection identity and failures

Connection discovery is a future addition, not an implicit action on opening the
preview. Docker can enumerate contexts with `docker context ls --format json` and
inspect a selected context. Podman can enumerate destinations with
`podman system connection list --format json` and report engine information with
`podman info --format json`.

For Docker, pin native calls with the selected `--context CONTEXT`; for Podman,
use the selected connection when applicable. Local rootless/rootful access and
remote destinations must be represented explicitly. Do not call `context use`,
change the user's default connection or request sudo as a side effect of selection.
Finding a binary does not prove socket access, daemon readiness or authorization.

A normalized service reference needs engine, connection identity, full object ID,
kind, optional Vults project and captured time/revision. Engine/group names are
labels, not unique identities. On one engine's error, preserve the other engine's
results and label the failed engine's previous capture stale. Do not erase known
services or turn an unavailable engine into a healthy empty list.

References: [Docker contexts](https://docs.docker.com/reference/cli/docker/context/ls/)
and [Podman connections](https://docs.podman.io/en/latest/markdown/podman-system-connection-list.1.html).

## 9. Alerts, history and diagnostics

Observation events can come from bounded polling initially; event streams are an
optional refinement. Normalize differences against the previous snapshot. The
first capture is a baseline, not an alert for every existing stopped container.
Use stable event keys/topics and revisions so repeated polls do not repeat an
alert and recovery supersedes its failure. Distinguish deliberate stop/pause from
an unexpected failure or unhealthy configured check.

Per-project notification preferences affect delivery, not inspection or diagnostic
truth. Native persistence and feature activation stay in the existing Settings
owner; the preview edits memory only. Seen acknowledgement must expire on a
meaningful state change, not just a later timestamp.

Keep engine observations, user requests, confirmed operation outcomes and failures
separately attributable. Reuse existing local audit/trace storage rather than
inventing a second global Activity dashboard. Diagnostic export combines bounded
snapshots and selected logs into reviewed text; it is not `docker export`, an
unfiltered inspect dump, automatic telemetry or an engine backup.

## 10. Zeca handoff and action proposals

### Current path to preserve

Production `createConnections` receives a Zeca-enabled ask callback. The callback
in `render.ts` opens Chat and calls `ChatPanel.draft(text)`. `draft` appends to the
text already typed; it does not send it. If Zeca is off, that callback is absent.
The prototype models this handoff with an editable dialog and a simulated-send
message, rather than implementing another chat.

### Context to prepare

Use a bounded, immutable capture containing:

- Selected connector, engine, connection and full service ID; human name and optional project.
- Capture time/revision and freshness; endpoint, group/config origin and image where known.
- Process state, exit/failure facts, configured health and independent HTTP response state.
- Relevant resource sample, mounts, port mappings and recent changes.
- Selected bounded logs, their source/window and any truncation or missing-data indication.
- The user's editable question and intended project folder only when that folder is already known.

Select fields relevant to the question. Do not automatically attach all containers,
all project files, raw environment values or every diagnostic. Logs, labels and
server responses are untrusted source material, not instructions for Zeca or the
application. Current plaintext `ask(text)` does not yet implement retained typed
source binding: that is a narrowly scoped addition, not an existing guarantee.

### Conversation behavior

1. Resolve the selected capture and prepare an editable draft using the existing Chat handoff.
2. Preserve an existing draft and explicitly retain this service's source reference.
3. Send only when the user submits the conversation through the existing Chat flow.
4. Return an explanation with observed facts, uncertainty and proposed next steps.
5. If needed, request a scoped fresh read through verified native tools; do not pretend stale logs are live evidence.
6. Represent any proposed mutation with a typed action, exact connection/object ID, impact and fresh-state check. Reuse existing action/permission ownership before execution.
7. Record outcomes and optionally attach them back to the same conversation; do not make another service selection rewrite its earlier source.

Zeca cannot answer permission cards, broaden access from a log, install a CLI,
rewrite Settings or automatically execute a copied diagnostic command. Appearance
and a project label grant no extra authority. Do not silently save these captures
to memory; use the existing explicit memory workflow if the user requests that.

Example question: investigate the selected Redis failure using its Permission
denied log and mounted `/data` volume. Zeca may explain that permissions are a
candidate cause and suggest an inspection. A restart alone does not prove that
cause was fixed, and a broad chmod proposal is not an authorized repair.

Existing boundaries: [human permissions](../../docs/adr/0004-a-human-answers-permissions.md),
[surface and Settings ownership](../../docs/adr/0021-four-sizes-one-nest.md),
[Chat implementation](../../ui/src/island/chat.ts) and
[Connections handoff](../../ui/src/surfaces/nest/render.ts).

## 11. Future ideas: not implemented controls

| Possibility shown in the review frame | What it would use | Limit before calling it complete |
| --- | --- | --- |
| Real group execution | Compose/pod commands above | Exact membership, command supervision and per-service outcomes |
| Dependency map | `docker compose config --format json`, real configuration and known associations | Startup declarations are not a complete runtime topology; show unknown relationships |
| Resource history / thresholds | Repeated normalized `stats` samples | Actual sampling interval, units, retention and unavailable values |
| Contexts / sockets | Context and connection discovery above | Explicit target selection and backend validation without changing defaults |
| Image/volume storage inspection | `docker system df -v`; `podman system df -v`, plus scoped image/volume inspect | Shared storage accounting differs; do not convert estimates into an automatic prune |
| Compose Watch | `docker compose -f FILE -p PROJECT watch` | Verified development configuration; rebuild/sync/start can mutate resources |
| Podman Quadlet | `podman quadlet list/print` and selected systemd units | Capability/version check and separately reviewed persistent service configuration |

References: [Compose dependency order](https://docs.docker.com/compose/how-tos/startup-order/),
[Docker storage](https://docs.docker.com/reference/cli/docker/system/df/),
[Compose Watch](https://docs.docker.com/reference/cli/docker/compose/watch/) and
[Podman Quadlet](https://docs.podman.io/en/latest/markdown/podman-quadlet.1.html).

## 12. Validation and cleanup

Run `node design/connections-local/check.mjs`. The check exercises the complete
browser proposal, verifies the scoped documents and absence of Flock, checks
reviewed diagnostic download/clipboard, CLI targets and capability guards, and
ensures the page makes no HTTP requests or JavaScript errors. It creates captures
in a unique temporary directory and removes them in `finally`, including failures.

This validates the prototype, not live CLI behavior, Tauri permissions, connected
engines or model execution. CLI flags were checked against official documentation
and the installed tools' help; no real container was modified. Native implementation
needs engine-specific recorded-output tests, disconnected/stale/capability cases,
exact-target action checks and integration with the existing Chat/Settings owners.

Keep only `index.html`, `design.md`, `functions.md`, `AGENTS.md` and `check.mjs` for
this exploration. Shared dependencies, tracked sprite sources and other design
work are not disposable caches. No commit is requested.
