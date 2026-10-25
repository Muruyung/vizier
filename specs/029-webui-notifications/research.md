# Research: WebUI Notifications

**Feature**: `029-webui-notifications` | **Researched**: 2026-10-25 | **Plan**: [plan.md](./plan.md)

The existing away feed (`specs/024-webui-rewrite/`, `src/channels/http/api/v1/overview/feed.rs`) is the reference for wording, destinations and visibility. This document records why the notification inbox differs from that read-time projection and how each source is handled.

## R1 — Persist the inbox; do not derive it from the feed

**Decision**: Add a durable `notification` row per notable event and a `notification_read` row per person who has read it. The old read-time feed projection is removed.

**Rationale**: The spec needs things a projection cannot reliably provide:
- **Stable identity** for read state, per-row actions and keyset paging.
- **History that survives the source**: a task run whose task is later deleted must remain readable; an agent start failure must remain in the inbox after the agent recovers; the old feed pinned only *currently* failed agents and a deleted task took its run with it.
- **Ready/failed setup outcomes**, which exist only in the in-memory setup board for 5 s.
- **A cheap unread count** for a polled badge. Counting a derived feed would require running every source aggregate on every poll and subtracting read marks.
- **One row per event per person-independent inbox**, not one row per person.

**Alternatives rejected**:
- *Keep the feed and store read marks by event id*: no durable setup outcomes, no history after source deletion, and the count still scans every source.
- *One notification row per person per event (fan-out at creation)*: write amplification, stale permissions, and an expensive fan-out for every agent event; read state is the only per-person part.
- *A materialized view/table refreshed by a scanner*: a second projection to maintain, with latency and a watermark for every source; the event sites already know when they complete.

## R2 — One creation path, idempotent by `event_key`; channel replies projected on read

**Decision**: A single `Notifications` service creates rows from the source completion sites. Every row has a deterministic `event_key`; inserts are `INSERT OR IGNORE`. The only aggregate source, channel replies, is projected from `session_history` on notification read/count, under a watermark.

**Rationale**: Task runs, dreams, jobs, integrations and setup outcomes each reach a terminal transition in one place. Hooking there gives immediate notifications and exactly-once semantics through the unique key. Channel replies have no "completion" code path — they are a `GROUP BY agent, channel, hour` over history — so a read-time projection with a watermark avoids both a background daemon and per-response write amplification.

**Alternatives rejected**:
- *A periodic projector for every source*: adds latency, a watermark per source, and a task to supervise; only channel replies genuinely need aggregation.
- *A transport event bus*: a new subsystem for five call sites; the service handle is simpler and testable.
- *Database triggers*: couples storage DDL to wording, agent names and permissions.

**Error rule**: a notification write is best-effort. A failure is logged and never fails the task run, dream, background job, integration write or turn that caused it (the source event has already happened).

## R3 — Unread is the absence of a read row

**Decision**: `notification_read(user_id, notification_id, read_at)` exists only for read notifications. Mark read inserts, mark unread deletes, and mark-all is one `INSERT … SELECT` over the visible unread set.

**Rationale**: New notifications are unread by default with no write, read state is per person, mark-unread is a delete, and mark-all cannot accidentally mark a notification created after the snapshot read. The counts query is one `LEFT JOIN` and the filter is `r.read_at IS NULL` / `IS NOT NULL`.

**Alternatives rejected**:
- *A `read` boolean on the notification*: read state would be shared by everyone, which the spec forbids.
- *A read-through timestamp per person*: efficient for "mark all", but cannot represent marking a single notification unread or reading an older one while leaving a newer one unread.
- *A row per person per notification with a boolean*: fan-out at creation; the read table is written only when someone acts.

## R4 — Setup: live progress reuses the setup poll, outcomes are notifications

**Decision**: Live setup (starting/downloading, download bytes/status, ETA) uses the existing `GET /utils/setup-state` and `useAgents` poll, not a notification row. `SetupState` gains the agent's `name`. The supervisor creates:
- one `agent_setup_ready` notification after a successful start **if that build downloaded at least one artifact or the episode is recovering from a failure**;
- one `agent_start_failure` notification when a new failure episode begins, shared with the start-failure category (the setup board does not create a second failure).

**Rationale**: Live progress changes every second and has no read semantics; making it a notification would produce an unclearable badge and a write per tick. The setup board is deliberately ephemeral; only the terminal outcome deserves history. A ready notification after a download keeps config saves and quick restarts out of the inbox, while a recovered failure still gets a visible "ready" close. One failure source prevents the setup failure and supervisor failure from duplicating.

**Implementation notes**:
- `SetupBoard` records a per-agent monotonic **download epoch**; `BuiltAgent::build`/`build_quietly` snapshot it before and after and carry `setup_downloaded` and a `build_id`. The supervisor consumes the flag; the build id keys the ready notification.
- The supervisor tracks a **failure-episode id** opened at the first failure after a successful start; retries reuse it, and a successful start clears it, so a later failure is a new episode. Only the first failure of an episode notifies. A successful build after a failure qualifies for a ready notification even if the retry itself downloaded nothing.
- A create build can show live setup to its owner (the endpoint already handles `owner_id`), but no notification is recorded before the agent is persisted. A build stopped by shutdown/delete does not notify failure.

**Alternatives rejected**:
- *Persist every setup tick*: noise, no read semantics, high write rate.
- *A new SSE/WebSocket progress stream*: the existing 1 s poll already meets the 2 s requirement and is used by the chat.
- *Let the setup board write notifications directly*: couples the in-memory board to storage/wording and makes the supervisor's failure dedupe harder.
- *Notify every ready build*: config saves/restarts would fill the inbox; only a build that downloaded something, or a recovery from a failure, is notable.

## R5 — Store once, filter by visibility at read

**Decision**: A notification is stored once with its `agent_id`; the list, count and read endpoints resolve `visible_agents` first and pass that whitelist to storage. There is no per-user copy and no owner column.

**Rationale**: The existing `visible_agents` rule covers ownership, sharing and permissions in one place. Checking at read makes revocation immediate and keeps the row count independent of the number of people. A person who gains access later sees retained notifications for that agent as unread, which matches the spec's visibility rule.

**Alternatives rejected**:
- *Fan-out at creation*: must be replayed on every permission/sharing change, multiplies rows, and can leak stale notifications.
- *Filter only at creation*: revoked people would keep seeing history.
- *Store `owner_id`/sharing on the notification*: duplicates a rule that already exists and can drift.

## R6 — A slide-over panel, not a route; polling within the latency budget

**Decision**: The Notifications item is a button above Settings; it opens a right-hand `SlideOver` (full-screen at phone widths). The panel contains the live setup section, sticky All/Unread/Read filters with counts, “Mark all read”, and the paged list. The store polls `GET /notifications/count` every 15 s while the tab is visible and refreshes the open list while it is open; setup progress uses the existing 1 s poll. No WebSocket, SSE or personal broadcast is added.

**Rationale**: The primary use is a glance without leaving the current screen; the app already uses `SlideOver` for detail surfaces and the design system has `FilterTabs`, `ListItem`, `Sigil`, `EmptyState`, `Skeleton` and `LoadMore`. Polling is simple, works across tabs and meets the spec's 30 s freshness budget (15 s poll) without adding a session-independent transport stream.

**Alternatives rejected**:
- *A full `/notifications` page*: leaves context and adds route/back-link state; the panel covers the content and the setup section.
- *A small dropdown*: too small for filters, setup cards and 30-day paging.
- *WebSocket/SSE for live badges*: the transport's broadcast is session-scoped; a personal notification stream would need a new subscription/lifecycle. Polling is sufficient for the required 30 s.
- *Category tabs or search*: explicitly out of scope in this release.

## R7 — Retire the old feed completely

**Decision**: Remove `GET /overview/feed`, `src/channels/http/api/v1/overview/feed.rs`, `FeedStorage`, `src/storage/feed.rs`, `src/storage/sqlite/feed.rs`, `AwayFeed.tsx`, `getFeed`/`Feed` client types and Home's right-hand column. Move the wording and cursor helpers into `src/notifications/`.

**Rationale**: Keeping both mechanisms would mean two definitions of "what happened", two sets of wording, two pagination cursors and a permanently dead endpoint. The notification rows already cover every old category plus setup outcomes. This is a deliberate breaking HTTP/WebUI change, acceptable pre-1.0 and called out in the commit/changelog.

**Alternatives rejected**:
- *Keep `/overview/feed` as a compatibility alias*: dead code, drift, and a reason to keep the feed queries.
- *Keep the Home panel as a preview*: duplicates the inbox and contradicts the spec's single inbox.
- *Backfill history as unread notifications*: the spec explicitly starts the badge at zero; the reply projection likewise starts at the last closed hour of activation.

**Visit tracking**: `record_visit`/`user.last_seen_at`/`previous_visit_at` become unused once the feed is gone. Removing the columns is a schema change with no user value; leave them untouched and add a #82 cleanup note rather than smuggling a migration into this feature.

## R8 — Channel replies: project closed hourly batches on read

**Decision**: Store a single global watermark `notifications.replies_through` (the last closed hour projected). On list/count/read-all, `Notifications::project_replies(now)` projects every closed hour after the watermark, capped to the 30-day window, as one notification per `(agent, channel, hour)` with `event_key = reply:{agent}:{channel}:{hour}`. On first activation the watermark is set to the last closed hour and nothing historical is imported.

**Rationale**: The old feed grouped replies per conversation-hour and included the partial current hour. The spec says a batch is complete; the inbox waits for the closed hour. A watermark avoids a 30-day scan on every poll, and a unique key makes concurrent projections idempotent. The projection is global (all agents), not per person, so visibility is still applied at read.

**Alternatives rejected**:
- *Insert on every Discord/Telegram response*: write amplification and a race to update the hourly count; a partial batch would also be notified.
- *A periodic projector task*: an extra supervised task for one source; read-time projection has no daemon and is naturally lazy.
- *Derive replies at read without storing them*: read state and counts would need a second mechanism, and the notification would vanish if the history row were deleted.

## R9 — Design system: extend primitives, design first

**Decision**: Make the small design changes in `design/` first, then consume them in `webui/`, then bump the pins in the root:
- `Bell` icon.
- `NavItem` optional unread count and a separate busy dot; `Sidebar` `notifications` prop above Settings.
- `ListItem` `action` slot rendered as a sibling of the row's link/button.
- `AgentStartingCard` downloads slot and panel width.
- Stories for each new state.

**Rationale**: The notification row needs a mark-read button that is not nested inside the row's link/button (invalid interactive nesting and unreliable event handling). The action-as-sibling extension keeps `ListItem` as the one row primitive and is reusable. The bell/badge/busy states belong with the nav primitives. The setup card already owns the progress visual; it only needs a downloads slot and a width that fits a phone.

**Alternatives rejected**:
- *A one-off WebUI row*: drifts from the design catalogue and repeats `ListItem`'s layout.
- *A trailing button inside the current `ListItem` link*: invalid HTML and ambiguous clicks.
- *Category icons in every row*: the spec's row is agent/what/when/failure; the wording already identifies the category, and the sigil keeps the row scannable.

## R10 — Retention, indexes and query shape

**Decision**: List/count only `created_at >= now - 30 days`; purge older rows at startup and once a day from the scheduler's existing minute tick. Index `notification(agent_id, created_at DESC, id DESC)` and `notification(created_at DESC, id DESC)`; the read table's primary key `(user_id, notification_id)` covers the join. Page size 20 (max 50) with a `(created_at, id)` keyset; over-fetch one to compute `next`.

**Rationale**: 30 days bounds both storage and attention. The keyset is stable when new notifications arrive and when timestamps collide; `AUTOINCREMENT` prevents id reuse. Counts are one grouped query over the visible whitelist. The reply projection avoids a scan when the watermark is current.

**Alternatives rejected**:
- *Infinite retention*: unbounded growth and a badge that never forgets.
- *Offset paging*: shifts when new rows arrive.
- *A separate counts table*: another thing to keep in sync; the indexed counts are cheap.
- *UUID primary keys*: ordering then needs another column; the integer id plus `event_key` already gives stable identity.

## R11 — Testing strategy: unit + API/UI e2e + marked live steps

**Decision**:
- **Rust tests** own storage semantics (idempotency, counts, filters, cursor, retention, visibility, agent cascade, reply projection), wording/hrefs, setup download epoch, failure/ready policy, task/dream/job/integration creation.
- **API/dummyplug quickstart** covers the inbox end to end: create an integration and a one-time task on a dummyplug agent, use `execute_python` to launch a background job, then exercise list/count/read/read-all and restart persistence.
- **SQLite fixture** inserts closed-hour Discord/Telegram response rows, then asserts the reply projection creates exactly one notification per conversation-hour.
- **Playwright** (`webui/e2e/notifications.spec.ts`) covers the desktop sidebar/badge/panel/filters/read actions/Home removal and the phone drawer/full-screen sheet.
- **Live steps** (clearly marked): an Ollama model download for live setup progress/ready, and a real provider dream for the dream notification, because dummyplug produces no journal entries and cannot download a model.

**Rationale**: The constitution requires a dummyplug e2e path for agent-observable surfaces; the notification API, task/background sources and UI are observable there. Setup downloads and dream journaling are not reproducible offline, so they get focused Rust tests plus live/manual steps rather than a misleading dummyplug claim.

**Alternatives rejected**:
- *Only unit tests*: no proof the panel and HTTP surface work.
- *Only Playwright*: cannot inspect storage/idempotency/retention and depends on a live server.
- *Pretending dummyplug can download/dream*: would make the quickstart dishonest.

## R12 — Migration and compatibility

**Decision**: New tables only (`CREATE TABLE IF NOT EXISTS`); no data migration and no changes to existing tables. The `/overview/feed` endpoint and Home panel are removed as a deliberate breaking change. No compatibility alias or legacy shim is added. The feature starts with an empty inbox/badge; channel replies begin at the last closed hour of activation. The unused visit tracking is left in place and tracked in #82.

**Rationale**: The project is pre-1.0, the spec replaces the surface, and a backfill would flood the badge with history the spec says must not appear unread. Adding new tables is the lowest-risk storage change; the only existing-table edit is deleting notifications in `delete_agent`, which is a data-lifecycle rule, not a migration.

**Alternatives rejected**:
- *Keep the feed endpoint/UI*: duplicate source of truth.
- *Backfill*: violates FR-039 and the "badge starts at zero" assumption.
- *Drop the visit columns*: unrelated schema churn; tracked for a later cleanup.

## R13 — Ordering, concurrency and exactly-once

**Decision**: A notification's public order is `(created_at, id)` descending; the cursor is base64 JSON of both; `event_key` is unique and inserts are `INSERT OR IGNORE`. Mark-all is `INSERT OR IGNORE … SELECT` over the current visible unread set; new rows after that snapshot stay unread. The reply projection is idempotent and serialized by SQLite writes; concurrent list/count calls may duplicate work but not rows. Agent deletion removes notifications in the same `delete_agent` call; the service also refuses to insert for an agent that no longer exists.

**Rationale**: Same-timestamp events (task runs closing together, setup ready at the same second) must page deterministically. The unique key is the exactly-once guard for retried transitions (job recovery, task recovery, startup sweeps). Refusing deleted agents closes the delete/insert race; the transaction still removes existing rows.

**Alternatives rejected**:
- *Ordering by timestamp alone*: ties page unpredictably.
- *A process-wide creation lock*: unnecessary; SQLite serializes writes and the unique key handles duplicates.
- *Fan-out read rows on creation*: more writes and stale permissions.

## R14 — Browser push is deferred

**Decision**: 029 is in-app only. It adds no service worker, no VAPID key storage, no push subscriptions and no background delivery. Browser/Web Push becomes a separate feature (for example `030-browser-push`) once this inbox lands.

**Rationale**: True browser push is a delivery channel with infrastructure the app does not have yet: a service worker and PWA manifest, an HTTPS origin (service workers do not work on a plain LAN `http://192.168…` address), VAPID key generation/storage/rotation, a per-device subscription table (endpoint, `p256dh`, `auth`, user, failures/pruning), RFC 8291 payload encryption, and a sender that resolves which people may see the agent. It also needs a noise policy: pushing every answered task, dream and hourly reply is unusable, so a later feature must decide which categories push (failures, background reports, setup outcomes, or a digest) and add the preference UI the current spec deliberately omits. iOS only supports Web Push for a Home-Screen web app (16.4+), not a normal Safari tab. Because notifications are already persisted here, push is additive: a best-effort `PushDelivery::notify(&notification)` can hook after `Notifications::record` without changing the inbox, API or read state.

**Alternatives rejected**:
- *In-tab OS notifications via the `Notification` API*: still requires a secure context and only works while the app is open/backgrounded; a small follow-up at most, not the same as push.
- *Adding push to 029 now*: would balloon the storage/API/UI scope and the deployment requirements (HTTPS, PWA, external push service) for a feature the spec explicitly scoped out.
- *Email or chat delivery instead*: also external and a different product surface; no better fit for “the browser is closed”.

## Amendment after implementation — the bell lives in the page header

The first implementation put the Notifications entry in the sidebar footer above Settings (R6, R9). That left the entry on the left and the panel sliding in from the right: the control and the surface it opened were on opposite edges, which read as a navigation item that behaved like a corner utility. The bell now sits at the right end of every page header (the shared `PageFrame`/`SettingsFrame`, Home, the chat header, and the phone top bar) so it shares the panel's edge. The sidebar goes back to navigation only; `NavItem`/`Sidebar` are unchanged, and the count/busy states moved onto `IconButton`. The rest of R6 — a slide-over, not a route, with a 15 s poll — stands.
