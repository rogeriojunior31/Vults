# Connections: local environment

This specification accompanies [index.html](index.html). It proposes one addition
to the existing Connections surface: local HTTP checks and Docker/Podman services.
It does not propose a redesign of the rest of Vults. Implementation details and
all CLI mappings are in [functions.md](functions.md); agent scope is in [AGENTS.md](AGENTS.md).

## Baseline and intended change

The current Connections renderer already owns connector chips, sync status,
filters, rows, a selected inspector, commands to copy and Ask Zeca. Production
currently registers GitHub. Keep those owners and GitHub's Inbox, Pull requests,
Actions, Releases and merge behavior.

The proposed local connection adds a project overview and technical inspection
inside that same surface. The prototype's outer review header, demonstration
scenarios, documentation links and future-idea cards are review tools, not new
application pages. Room/Full only demonstrate widths; they do not replace the
native shell or its four-size specification.

Flock is not used in this preview and has been removed from its navigation. The
production Flock remains outside this proposal. Tasks, Activity and Settings are
disabled context entries; the Chat entry only opens the sample Zeca draft.

## Terms and identity

- **Service:** the general item being observed, either a container or an HTTP server.
- **Server:** an independently monitored HTTP endpoint. It can belong to Vite,
  Node, FastAPI or an application inside a container. Monitoring an address alone
  does not discover a PID, application logs or process control.
- **Associated HTTP check:** an endpoint attached to an existing container item.
  It has its own response state and is not another copy of that service or its
  image-defined healthcheck.
- **Project:** an optional Vults grouping. Unassigned services appear under
  **Without project**; that grouping is not counted as a configured project.
- **Compose project / Podman pod:** an engine-owned group with discovered metadata.
  A similar name or Vults project label is not proof that a group exists.
- **Connection:** the exact engine target and user identity. Docker contexts and
  Podman rootless, rootful and remote connections must not be silently combined.

## Views and interactions

| View | Purpose | Interactive scope |
| --- | --- | --- |
| Projects | Understand the environment before investigating | Counts, failures-first service highlights, project filter, inspector shortcuts, alert preferences and diagnostic export |
| Problems | Find unresolved failures | Service selection, editable Zeca draft, mark an alert seen |
| Servers | Inspect independent HTTP checks | URL, response facts, linked sample logs or check history, add a check with optional project |
| Containers | Inspect and control existing containers | Combined engine/project/search filters, runtime and health, CPU/RAM/restarts, configuration, logs, mounts, lifecycle confirmation and CLI toolbox |
| Ports | Locate endpoints and published mappings | Runtime port mappings and associated HTTP checks, each opening its owning service |
| Events | Understand changes and user actions | Project-filtered history and per-container history |

Project cards show up to three highlights, with failures first. Their inspector
shortcut opens that project's containers, or its servers when no containers exist.
Counts cover all matching services, not only the displayed highlights. Marking an
alert seen dismisses that problem until its state changes; the service remains
inspectable in Servers/Containers and still contributes to actual failure totals.

A selected container separates process state, healthcheck and HTTP response.
Running is not synonymous with healthy; no configured check means **Not configured**.
A stopped or paused service is distinct from an unexpected failure. A restart can
fail again and must preserve the underlying error rather than manufacture recovery.

The inspector's first level keeps state and actions visible. Less frequent
configuration facts expand on demand. Logs, ports/volumes and events have separate
inspector tabs. Narrow previews drill into the selected service and offer Back to
list, preserving keyboard navigation. The content region scrolls independently
and offers **See more** when additional content is below it.

## HTTP monitoring

A check may be independent or associated with one existing container. The project
is optional for an independent check. Association retains the container's project
and does not create another service. The prototype accepts HTTP/HTTPS on localhost,
127.0.0.1 or IPv6 loopback, without credentials in the URL.

Native implementation needs explicit method, expected response, timeout and
schedule defaults. A basic HTTP check provides response/latency/failure facts;
process logs and PIDs require a separately known source. Failed requests and port
mapping facts do not identify a conflicting process automatically.

## Lifecycle and CLI toolbox

Start, stop, restart, pause and resume apply to an existing selected container.
The confirmation identifies engine, container and impact. Cancel changes nothing.
No deletion, prune, image replacement or volume editing is included.

The CLI toolbox prepares commands for inspect, bounded/followed logs, processes,
resource snapshots, filesystem changes and healthcheck inspection. It displays
capability/state restrictions rather than promising equal behavior across engines.
Docker health inspection is not a manual healthcheck execution. Podman's manual
healthcheck requires a configured check and a running container.

Compose and pod dialogs show sample members and group scope, then prepare logs,
start, stop or restart commands. Compose also has a dry-run initialization preview.
These group dialogs copy commands only; they do not simulate per-member execution.
The shown Compose path and groups are illustrative, not detected from this machine.

## Alerts and diagnostics

Alert preferences are per project/group: failures and recoveries, failures only,
or silent. Silence changes notification preferences, not service truth. The demo
saves preferences in memory until reset and sends no notifications.

A diagnostic is an editable text snapshot containing the selected project's
services, states, images, mounts, HTTP associations and logs. Copy/download uses
that reviewed text. It is not an engine filesystem export or a running investigation.

## Zeca

Ask Zeca prepares a question using the selected service's captured context and
logs. The user can inspect and edit it before sending. Production should use the
existing Connections ask callback and Chat draft behavior, preserving any text
already typed. Selecting another service must not silently retarget an existing
conversation. Zeca being off removes the relevant actions.

Context is evidence, not authority. A diagnosis may propose an action; execution
must go through the existing user action/permission path with a fresh, exact target.
The prototype's Send button only displays a simulated-send message.

## Demonstration versus implementation

| Behavior | Present in this artifact | Production work still required |
| --- | --- | --- |
| Views, filters and inspector | Interactive, in-memory examples | Typed local-service view model and actual snapshots |
| Container lifecycle | Confirmed state simulation, including persistent failures | Native execution, capability checks, fresh state and recorded outcomes |
| Toolbox / Compose / pod | Editable option selection and command copy | Supervised native processes and authoritative group discovery |
| HTTP checks | Validated draft; optional project and container association | Native requests, configuration ownership, scheduling and response history |
| Alerts / Events | Preferences and sample history | Existing notification, deduplication, persistence and audit integration |
| Diagnostic | Working text review, clipboard and download | Bounded real captures and redaction |
| Zeca | Editable context and simulated send | Existing Chat handoff and retained source identity |
| GitHub | Simplified reference fixtures | Keep the existing production implementation |

## Future possibilities

These are options to evaluate after implementing the scoped connection:

1. Real group execution with discovered membership and a result per affected service.
2. Dependency/impact views derived from configuration, not invented topology.
3. Resource history and threshold alerts from actual measured samples.
4. Explicit context/socket selection, with connection identity visible throughout.
5. Storage inspection for images and volumes; cleanup needs its own scoped flow.
6. Compose Watch for development and Podman Quadlet for Linux systemd management.

## Validation and files

Open `index.html` in a browser from this repository checkout. It references the
current Nest CSS and Zeca image, so moving this directory outside the repository
alone does not produce a standalone asset bundle.

Run `node design/connections-local/check.mjs`. The check covers browser flows,
command targets, cancellation, failure persistence, optional projects, HTTP
association without duplication, diagnostics, input escaping, keyboard focus,
scroll navigation and 1280/760/390-pixel layouts. It verifies no browser network
requests or JavaScript errors and confirms Flock is absent. Captures are temporary
and removed even if validation fails.

No application code or dependency changes are required to review this proposal.
No commit is part of this work.
