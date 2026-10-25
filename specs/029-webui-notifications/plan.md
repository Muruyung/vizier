# Implementation Plan: WebUI Notifications

**Branch**: `029-webui-notifications` | **Date**: 2026-10-25 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/029-webui-notifications/spec.md`

## Summary

Replace Home's “Since you were away” panel with a per-person **notification inbox**. A bell above Settings carries an unread count and a separate setup-activity indicator; it opens a right-hand slide-over (full-screen on phones) with **All / Unread / Read** filters, per-row read/unread actions, “Mark all read”, and 30 days of persistent notifications. Live agent setup progress (build, model/embedding/STT/TTS downloads) appears above the filters and is deliberately *not* a read/unread notification; a setup that downloaded something or recovers from a failure produces one ready notification, and any start failure produces one failure notification shared with the start-failure category. The detailed starting card leaves chat, leaving a compact queued-messages line and the existing actionable failure banner.

Seven decisions carry the plan (details in [research.md](./research.md)):

1. **Persist the inbox, not a derived view** (R1). One `notification` row per notable event, created at the moment the event completes, plus a `notification_read` row only when a person reads it. This gives stable identities, per-person read state, retention that does not depend on the source row surviving, and a home for otherwise-ephemeral setup outcomes.
2. **One creation path, idempotent by `event_key`** (R2). Task runs, dream cycles, background jobs, integration changes, agent ready and agent start failure call the one `Notifications` service; channel replies are projected from `session_history` on read, because an hourly batch is the only aggregate source. No event bus and no periodic scanner.
3. **Read state is a row, unread is the absence of one** (R3). Mark read inserts, mark unread deletes, mark-all is one `INSERT … SELECT`. Counts and filters are one indexed query against the visible agents.
4. **Setup progress reuses the existing poll** (R4). `/utils/setup-state` already reports phases and downloads and already filters by agent visibility; it gains the agent's name. The supervisor emits the ready outcome when the build downloaded something **or** the episode is recovering from a failure, and the failure outcome once per failure episode, so quick restarts fill nothing, a recovered setup still closes the loop, and setup failure does not duplicate the start-failure notification.
5. **Notifications are stored once and filtered by visibility at read** (R5). No per-user fan-out; `visible_agents` is evaluated on every list/count/read, so revoked access disappears immediately.
6. **The inbox is a slide-over, not a route** (R6). It opens over the current screen, preserves context, uses the existing `SlideOver`, and becomes full-screen at phone widths. The store polls the unread count every 15 s (within the 30 s budget) and the open list while it is open; setup uses the existing 1 s poll. No WebSocket/broadcast addition.
7. **Retire the old feed completely** (R7). Remove `/api/v1/overview/feed`, the `FeedStorage` trait/queries and `AwayFeed`; move the wording and cursor helpers into the notification module. This is a deliberate breaking HTTP/WebUI change (pre-1.0), replaced by `/api/v1/notifications`.

## Technical Context

**Language/Version**: Rust edition 2024 (single `vizier` binary, tokio multi-thread); TypeScript 5.9 / React 19 / React Router 7 (SPA) / Tailwind v4 in `webui/`, built on the `design/` submodule.

**Primary Dependencies**: No new Rust crate and no new WebUI dependency. Reuse `axum`, `rusqlite`, `chrono`, `uuid`, `serde`/`serde_json`, `utoipa`, `tracing`, `parking_lot`, `tokio`; reuse `@vizier/design` `SlideOver`, `FilterTabs`, `ListItem`, `Sigil`, `EmptyState`, `Skeleton`, `LoadMore`, `IconButton`, `TextLink`, `AgentStartingCard`, and the existing Zustand/API patterns.

**Storage**: Embedded SQLite. Two new tables, `notification` and `notification_read`; no existing table changes. The reply-projection watermark lives in the existing `state` key/value store. Retention is enforced at read (`created_at >= now - 30 days`) and by a startup + daily purge.

**Testing**: `cargo test` (storage idempotency/counts/cursor/retention/visibility/projection; wording and event keys; setup download epoch and supervisor policy; agent-delete cascade), `cargo clippy`; `cd webui && npm test` (store merge/mark-all/filter helpers) and `npm run typecheck`; `cd webui && npm run e2e` (a new notifications spec for sidebar, panel, filters, read actions, Home removal and the phone sheet); end-to-end: dummyplug steps in [quickstart.md](./quickstart.md) §1–§7, with live-Ollama/download and dream steps clearly marked, and a SQLite fixture for the channel-reply projection.

**Target Platform**: Linux, macOS and Windows via the single binary; modern evergreen browsers for the WebUI. No OS-specific code.

**Project Type**: Web application inside the existing single-binary Rust service (Rust backend + `webui` submodule on the `design` submodule).

**Performance Goals**: `GET /notifications/count` under 50 ms p95 with 10k retained notifications and 100 visible agents; the first notification page under 100 ms; reply projection does no scan when the watermark is current and at most 30 days of closed hours on catch-up; the WebUI badge updates within 15 s while visible (spec budget 30 s) and the live setup card updates within 1 s from the existing poll.

**Constraints**: 30-day retention; no network needed for any notification read or write; no new config surface; notifications are plain text from stored facts (never model output, never a secret or integration setting value); a notification write failure is logged and never fails the task/run/job/integration/turn that caused it; visibility is checked at every read; the old `/overview/feed` endpoint is removed; setup live state is not persisted and does not count toward unread; design → webui → vizier commit order for the submodule changes.

**Scale/Scope**: roughly 5 HTTP endpoints (list, count, set read, mark all, plus the existing setup-state), two tables, one `Notifications` service, one notification text/cursor module, one Zustand store, one slide-over panel, and a handful of small design-system extensions. Expected volume is bounded by 30 days of task runs, dreams, jobs, integrations and one row per channel-hour.

## Constitution Check

*GATE: passes before Phase 0 research; re-checked after design below.*

| Principle | Assessment |
|---|---|
| **I. Lean by default** | PASS. No new crate. No event bus, scheduler projector, or per-user fan-out. The inbox is two tables, one service and one existing panel primitive. Live progress reuses the setup poll, and channel replies reuse the existing `session_history` aggregate instead of a new background job. Retiring the feed removes code rather than adding a second source. |
| **II. DRY via traits** | PASS. `NotificationStorage` is one storage concern implemented by `SqliteStorage` and forwarded by `VizierStorage`, following the established pattern. Wording and destinations live in one `notifications/text.rs`; the cursor in one helper. Sources call one `Notifications` service; there is no category `match` in handlers or storage. The design system additions extend existing primitives (`NavItem`, `Sidebar`, `ListItem`, `AgentStartingCard`) rather than creating parallel ones. |
| **III. Self-contained runtime** | PASS. SQLite only; no external service, queue or cache. Reads and writes need no network. Setup downloads are the existing opt-in model behaviour, not a requirement of notifications. |
| **IV. Portability** | PASS. No OS/arch-specific code, no filesystem assumptions beyond the existing SQLite workspace, no new native dependency. |
| **V. Unified errors & observability** | PASS. Rust paths return `crate::Result`/`anyhow::Result` per storage convention; notification creation failures are `tracing::warn!` and never propagate into the event; no `unwrap` outside tests. The projection and purge log their outcomes. |
| **Workflow gates** | PASS. `cargo clippy`, `cargo test`, `npm test`, `npm run typecheck`, Playwright and the dummyplug/live quickstart are in the phases. Docs and OpenAPI are updated. The old feed removal is a deliberate breaking change, called out in the conventional commit. No migration or legacy shim is introduced; the unused visit tracking is noted for #82 rather than removed here. Design changes are made in `design/` first, then `webui/`, then the vizier pins are bumped. |

**Post-design re-check**: no violations. Watch items: (a) notification writes must stay best-effort and cannot fail the event that produced them; (b) the ready notification qualifies when this build downloaded something **or** the supervisor is recovering from a failure episode, while a quick restart with neither stays live-only; (c) the reply projection watermark must be set to the last **closed** hour on first activation, so the feature does not import historical replies as unread; (d) the `ListItem` action must be a sibling of the row's link/button, never a nested interactive control.

## Project Structure

### Documentation (this feature)

```text
specs/029-webui-notifications/
├── plan.md                         # this file
├── research.md                     # R1–R13 decisions and alternatives
├── data-model.md                   # tables, entities, event keys, state transitions
├── quickstart.md                   # dummyplug/API/Playwright validation + live setup notes
├── contracts/
│   ├── notification-api.md         # HTTP endpoints, payloads, errors, cursor, counts
│   ├── notification-storage.md     # trait, DDL, queries, idempotency, retention
│   ├── notification-sources.md     # each event source, wording, href, dedupe
│   └── notifications-ui.md         # panel/sidebar/row/mobile/accessibility contract
├── checklists/requirements.md
└── tasks.md                        # /speckit-tasks (not created here)
```

### Source Code (repository root)

```text
src/
├── schema/
│   ├── notification.rs             # Notification, NotificationDraft, Category, Tone, Filter,
│   │                               #   Counts, Page, cursor types
│   └── mod.rs                      # re-export
├── notifications/
│   ├── mod.rs                      # Notifications service: record + source methods,
│   │                               #   reply projection, retention
│   ├── text.rs                     # wording + hrefs for every category (moved from feed.rs)
│   └── cursor.rs                   # opaque (at, id) cursor encode/decode + tests
├── storage/
│   ├── notification.rs             # NotificationStorage trait + forwarding impl
│   ├── mod.rs                      # add trait to VizierStorageProvider; forwarding
│   ├── sqlite/
│   │   ├── notification.rs         # DDL helpers + queries + projection + tests
│   │   ├── mod.rs                  # init_notification_schema in init_schema
│   │   └── agent.rs                # delete an agent's notifications in delete_agent
│   ├── feed.rs                     # DELETE (queries move to notification storage)
│   └── sqlite/feed.rs              # DELETE
├── dependencies.rs                 # construct Notifications; startup purge; interrupted
│                                   #   task-run/job notifications; pass to BackgroundJobs
├── setup.rs                        # + name, download epoch, ready flag; tests
├── scheduler/
│   ├── mod.rs                      # task-run notifications on close/recover/dispatch failure;
│   │                               #   daily retention purge
│   └── dream/mod.rs                # dream-cycle notification after consolidation
├── agents/
│   ├── background/mod.rs           # reported/cancelled/undelivered/recovered notifications
│   └── supervisor.rs               # ready (downloaded) + start-failure episode notifications;
│                                   #   BuiltAgent carries setup_downloaded/build_id
├── integrations/service.rs         # integration/grant notifications after commit
└── channels/http/
    ├── mod.rs                      # OpenAPI components for notification types
    └── api/v1/
        ├── mod.rs                  # nest /notifications under auth
        ├── notifications.rs        # list, count, set read, mark all
        ├── utils.rs                # setup-state returns SetupState.name
        └── overview/
            ├── mod.rs              # remove the /feed route/export
            └── feed.rs             # DELETE

design/src/
├── icons/map.ts                    # + Bell (lucide)
├── components/NavItem/NavItem.tsx  # + optional badge count / busy dot
├── components/Sidebar/Sidebar.tsx  # + notifications entry above Settings
├── components/ListItem/ListItem.tsx# + action sibling outside the link/button
├── components/AgentStartingCard/AgentStartingCard.tsx
│                                   # + downloads slot, full-width in a panel
└── ...                             # stories for each extension

webui/app/
├── api/endpoints/notifications.ts  # types + list/count/read/read-all calls
├── api/endpoints/overview.ts       # remove feed types/getFeed
├── api/schema.d.ts                 # regenerated from OpenAPI
├── stores/notifications.ts         # panel state, counts, list, filters, read actions, polling
├── screens/notifications/
│   ├── NotificationsPanel.tsx      # SlideOver body: setup, filters, list, states
│   ├── NotificationRow.tsx         # ListItem + unread gutter + mark action
│   └── SetupActivity.tsx           # AgentStartingCard + download rows
├── screens/home/AwayFeed.tsx       # DELETE
├── screens/chat/AgentStateBanner.tsx
│                                   # compact starting line; failure banner unchanged
├── layouts/AppShell.tsx            # render panel, start polling, wire sidebar, close drawer
├── routes/home.tsx                 # remove the away panel and its column
├── lib/
│   ├── overview.ts                 # remove feedTime
│   ├── format.ts                   # feedTime's new home
│   └── notifications.ts            # merge/filter/count helpers + tests
webui/e2e/notifications.spec.ts     # desktop/mobile panel, filters, read, Home removal

docs/src/api-integration/rest-api.md # remove feed row/visit note; add Notifications section
CLAUDE.md                            # storage/HTTP/WebUI notes for the new subsystem
```

**Structure Decision**: the feature sits in the existing single binary and the existing WebUI-on-design submodule layering. The backend adds one domain service (`src/notifications/`) and one storage concern, and removes the old feed rather than keeping two event sources. The design system gets small extensions to primitives that already exist; the WebUI gets one store and one panel screen. No new route, no new dependency, no new service, no external process.

## Phases

### Phase 0 — Design system (design submodule, first)

1. Add `Bell` to the icon map.
2. Extend `NavItem` with an optional trailing count badge and a separate busy indicator that collapses to a dot on the rail; add stories for unread, busy, both and collapsed.
3. Extend `Sidebar` with a `notifications` prop (`unread`, `busy`, `onOpen`) rendered immediately above Settings; add stories for expanded, collapsed, unread and busy.
4. Extend `ListItem` with an `action` slot rendered as a sibling of the row's link/button, so the mark-read control is never nested inside an anchor/button; add a story.
5. Extend `AgentStartingCard` with an optional downloads slot and make it fit a panel width (`w-full max-w-324` or equivalent); add a story with several downloads.
6. Commit the design submodule; this commit is the first of the three-part cascade.

### Phase 1 — Notification domain and storage (Rust)

1. `schema/notification.rs`: `NotificationCategory` (`agent_start_failure`, `agent_setup_ready`, `task_run`, `dream_cycle`, `background_job`, `integration`, `channel_replies`), `NotificationTone` (`default`, `failed`, `success`), `NotificationDraft`, `NotificationItem`, `NotificationCounts`, `NotificationPage`, `NotificationFilter`; serde/utoipa derives; parse/as_str round trips.
2. `storage/notification.rs`: `NotificationStorage` with `insert_notifications` (idempotent on `event_key`), `list_notifications`, `notification_counts`, `count_unread`, `set_notification_read`, `mark_all_notifications_read`, `channel_reply_batches`, `purge_notifications_before`. Add it to `VizierStorageProvider` and forward every method from `VizierStorage`.
3. `storage/sqlite/notification.rs`: DDL (`notification`, `notification_read`, indexes), visibility-aware queries, keyset paging, counts, mark-all, reply aggregates, purge, tests. `init_notification_schema` runs from `init_schema`.
4. `storage/sqlite/agent.rs`: delete the agent's notification rows in the same `delete_agent` transaction (read rows cascade).
5. Tests: idempotent insert, unread/read/all counts, filter exclusivity, cursor with tied timestamps, visibility whitelist, mark-all covers unread beyond the page, mark-unread, retention window/purge, agent-delete cascade, channel-hour grouping and platform labels.

### Phase 2 — Notification service and source hooks (Rust)

1. `notifications/text.rs`: move the feed's wording and destination builders; add `agent_ready`, `agent_failed` and channel-reply text; test pluralization, quotes and hrefs.
2. `notifications/cursor.rs`: opaque `(at, id)` cursor with the feed's round-trip and bad-cursor tests.
3. `notifications/mod.rs`: `Notifications` constructed with the storage; `record` verifies the agent still exists, inserts best-effort, and logs failures. Source methods: task run, dream cycle, background job, integration events/grants, agent ready/failure. `project_replies(now)` reads/writes the `notifications.replies_through` watermark, projects only closed hours, and is idempotent; `purge_before` enforces retention.
4. `setup.rs`: add the agent name to the setup entry/state; add a monotonic per-agent download epoch and a "downloaded this build" result; tests for reset on a new top-level build and overlap handling.
5. `agents/supervisor.rs`: `BuiltAgent` carries `build_id` and `setup_downloaded`; after a successful start, record one `agent_setup_ready` when the build downloaded something or the supervisor is recovering from a failure episode. On a failure with no episode open, record one `agent_start_failure` with the error/retry; retries of that episode reuse it, and a successful start clears it so a later failure is a new episode. Skip a delete/shutdown stop. One notification per episode; no setup-board failure duplication.
6. `scheduler/mod.rs`: capture the task title/agent/slug when firing; after every successful `close_task_run`, record answered/no-response/interrupted. `recover` and the startup sweep do the same for runs they close. Dispatch failure records `no_response`.
7. `scheduler/dream/mod.rs`: after consolidation and before/at Idle, count the cycle's journal entries; record `dream_cycle` when > 0.
8. `agents/background/mod.rs`: `BackgroundJobs` takes the `Notifications` handle; record reported, cancelled, undelivered and recovered-reported outcomes. Never record `reporting`.
9. `integrations/service.rs`: after each committed write, record one notification per event and affected agent, plus grant events; resolve the actor name once; never include setting values.
10. `dependencies.rs`: construct `Notifications`; perform the startup purge and first-activation reply watermark; return the rows swept by the interrupted task-run/job sweeps and record them after the service exists; pass the handle to `BackgroundJobs`; add the startup orphan cleanup if a race could leave a notification for a deleted agent.
11. Tests: one per failure episode, no ready on a download-less restart, ready when the build downloaded, task title fallback, reply projection watermark/no backfill, integration event keys, job terminal-only.

### Phase 3 — HTTP API and docs (Rust + OpenAPI)

1. `api/v1/notifications.rs`: `GET /notifications` (filter/cursor/limit + counts), `GET /notifications/count`, `PUT /notifications/{id}/read` (`{read}` + counts), `POST /notifications/read-all` (+ counts). Every handler resolves `visible_agents` first, projects replies, and never returns another agent's notification.
2. Register the router under `require_auth`; add the schemas to `ApiDoc`; add `SetupState.name` to the setup-state schema. Remove the `/overview/feed` route and its OpenAPI path/schemas.
3. Update `docs/src/api-integration/rest-api.md`: remove the feed row and visit paragraph; add a Notifications section with filters, counts, read/read-all, cursor and visibility; note the setup-state name. Update `CLAUDE.md` with the storage/creation/panel notes.
4. Delete `overview/feed.rs`, `storage/feed.rs` and `storage/sqlite/feed.rs`; move the reply query into notification storage; remove the `FeedStorage` supertrait entry.
5. Run `scripts/openapi.mjs` to refresh `webui/openapi.json` and `webui/app/api/schema.d.ts`.

### Phase 4 — WebUI (webui submodule, after the design commit)

1. `api/endpoints/notifications.ts`: types (`NotificationItem`, `NotificationCounts`, `NotificationPage`, `NotificationFilter`) and list/count/read/read-all calls.
2. `stores/notifications.ts`: `open`, `filter`, `items`, `counts`, `next`, `loading`, `error`, `loaded`; open/close, filter change, first-page refresh + silent merge on poll, load-more, optimistic mark read/unread, mark all, count poll. `start()` polls counts every 15 s while visible and refreshes the open list; `reset()` on sign-out/mount.
3. `screens/notifications/`: the `SlideOver` panel with the setup section (from `useAgents.setups`/`samples`, filtered to starting/downloading), sticky filter toolbar with counts and “Mark all read”, the list, `NotificationRow` (fixed unread gutter, sigil, agent name, time, failed/success text, sibling mark action), `LoadMore`, skeletons, error/empty states. Use `ListItem` with the new `action` slot; the row selects the destination and marks read.
4. Wire `AppShell`: start the notification poll and the existing agents/setup poll; render the panel; compute `busy` from live setups; pass `{unread, busy, onOpen}` to `Sidebar`; on phone close the drawer first, then open the full-screen sheet; ensure focus returns to the entry. The entry is a button, not a route.
5. Remove Home's `AwayFeed` and its right-hand column; delete the component and the `getFeed`/`Feed` client types; move `feedTime` to `lib/format.ts` and its test. Home keeps the greeting, activity panel and court.
6. `AgentStateBanner`: replace the detailed `AgentStartingCard` with a compact line (starting, step/percent, messages queued, “View in Notifications”); keep the failed banner with Retry now / Open config and the connection banners. The panel is the only detailed setup view.
7. Tests: `lib/notifications.test.ts` for merge/filter/count/time; store tests for optimistic read/mark-all and filter membership; typecheck; `e2e/notifications.spec.ts` (desktop + mobile): entry/badge, open, filter, mark read/unread, mark all, Home has no away section, phone drawer → full-screen, focus/escape.

### Phase 5 — Cleanup, verification and release order

1. Delete the old feed endpoint/storage/component and its tests/docs; confirm no imports remain. Add a #82 note for the now-unused visit tracking (`record_visit`, `last_seen_at`, `previous_visit_at`) if it is not removed in this change; do not drop columns here.
2. `cargo fmt`, `cargo clippy`, `cargo test`; `cd webui && npm test && npm run typecheck && npm run e2e`.
3. Run the quickstart: dummyplug task/integration/background checks, API read/filter/count/retention checks, SQLite reply fixture, Home/sidebar/mobile checks; mark the live Ollama download and live dream steps as live-only.
4. Verify no network at startup/agent build (notifications add none), and that a notification write failure does not fail the event (simulate/log).
5. Commit order: design submodule → webui submodule → vizier root (backend, docs, gitlink bump), with `/cascade-merge` or the normal three-PR flow. The root commit flags the `/overview/feed` removal as a breaking API change.

## Complexity Tracking

No constitution violations. No new dependency, external service, background daemon, migration or legacy shim. The one deliberate breaking removal is the `/api/v1/overview/feed` endpoint and the Home away panel, replaced by `/api/v1/notifications` and the notification panel; it is called out in the commit and docs. The unused visit tracking is left in place (no schema change) and tracked in #82 for a later cleanup.
