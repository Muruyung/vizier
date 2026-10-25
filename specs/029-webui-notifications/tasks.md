---
description: "Task list for WebUI Notifications"
---

# Tasks: WebUI Notifications

**Input**: Design documents from `/specs/029-webui-notifications/`

**Prerequisites**: `plan.md` (required), `spec.md` (required), `research.md`, `data-model.md`, `contracts/`, `quickstart.md`

**Tests**: Included because `plan.md` defines the verification gates and the project constitution requires `cargo test`/`cargo clippy`/WebUI typecheck and the dummyplug e2e path. Add the tests with the implementation they cover; the Rust tests live in `#[cfg(test)]` modules in the named source files.

**Organization**: Tasks are grouped by user story so each story can be implemented and tested independently. Paths are relative to the repository root; `design/` and `webui/` are submodules.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: The user story from `spec.md` (`US1`–`US7`); omitted in Setup, Foundational and Polish
- Every task names an exact file path and, where relevant, the required constraint from `data-model.md`

## Path Conventions

- Rust backend: `src/`
- Design system: `design/src/`
- WebUI: `webui/app/`
- Playwright: `webui/e2e/`
- Docs: `docs/` and `CLAUDE.md`

---

## Phase 1: Setup & design primitives

**Purpose**: The shared visual primitives every story uses. Do this first because the design submodule must be committed before the WebUI consumes it.

- [ ] T001 Add `Bell` to the icon vocabulary in `design/src/icons/map.ts` (lucide import + map entry) and update the icon type/story coverage.
- [ ] T002 [P] Extend `NavItem` in `design/src/components/NavItem/NavItem.tsx` with an optional unread count badge and a separate busy indicator: expanded count pill, collapsed dot, accessible name; add unread/busy/both/state stories to `NavItem.stories.tsx`.
- [ ] T003 Extend `Sidebar` in `design/src/components/Sidebar/Sidebar.tsx` with `notifications?: { unread: number; busy: boolean; onOpen: () => void }`, render a `NavItem` immediately above Settings, include the count in the collapsed tooltip, and add stories for expanded/collapsed/unread/busy. (Depends on T002's `NavItem` props.)
- [ ] T004 [P] Extend `ListItem` in `design/src/components/ListItem/ListItem.tsx` with `action?: ReactNode` rendered as a **sibling** of the row's `<a>`/`<button>` (never nested); preserve existing rows and add a story with a trailing action.
- [ ] T005 [P] Extend `AgentStartingCard` in `design/src/components/AgentStartingCard/AgentStartingCard.tsx` with a `downloads?: ReactNode` slot and a width that fits the 400px panel content / phone (`w-full max-w-324`); add a story with several downloads.
- [ ] T006 Run `design/package.json`'s checks (`cd design && npm run typecheck && npm test && npm run build-storybook`); fix any catalogue/type errors; commit the design submodule as the first part of the three-part cascade (depends on T001–T005).

**Checkpoint**: the design catalogue has the bell, nav badge/busy, sidebar entry, row action and setup downloads; the WebUI can consume them.

---

## Phase 2: Foundational (blocking prerequisites)

**Purpose**: Schema, storage, service skeleton, HTTP contract and WebUI client/store shell. No user story can be implemented before this phase.

**⚠️ CRITICAL**: complete before any user story phase.

- [ ] T007 Create the notification domain types in `src/schema/notification.rs`: `NotificationCategory` (seven values: `agent_start_failure`, `agent_setup_ready`, `task_run`, `dream_cycle`, `background_job`, `integration`, `channel_replies`), `NotificationTone` (`default|failed|success`), `NotificationFilter` (`all|unread|read`), `NotificationDraft`, `NotificationItem`, `NotificationCounts` and `NotificationPage`; re-export from `src/schema/mod.rs`. Constraints: `event_key`/`summary`/`href` non-empty; `created_at`/`read_at` are Unix milliseconds; `event_key` is unique (storage enforces); API `limit` 1–50 default 20. Add serde/utoipa derives and `as_str`/`parse` tests.
- [ ] T008 [P] Add `init_notification_schema` to `src/storage/sqlite/mod.rs` and call it from `init_schema`: `notification` (`id INTEGER PRIMARY KEY AUTOINCREMENT`, unique `event_key`, `agent_id`, `category`, `tone`, `summary`, `href`, `created_at`) and `notification_read` (`user_id`, `notification_id REFERENCES notification(id) ON DELETE CASCADE`, `read_at`, primary key `(user_id, notification_id)`), plus `idx_notification_created` and `idx_notification_agent_created`. Do not alter existing tables; `PRAGMA foreign_keys=ON` is already set.
- [ ] T009 Add `src/storage/notification.rs` with the foundational `NotificationStorage` methods `insert_notifications`, `list_notifications` and `notification_counts`; add the trait to `VizierStorageProvider` and hand-forward those methods from `VizierStorage` in `src/storage/mod.rs`. Later story tasks extend this trait with read/mark-all (T024), reply batches (T039) and purge (T055), adding their forwarding at the same time so each checkpoint compiles. (Depends on T007's types.)
- [ ] T010 Implement `insert_notifications` in `src/storage/sqlite/notification.rs`: one transaction, `INSERT OR IGNORE` on `event_key`, skip a draft whose `agent_id` is no longer in `agent_config`, validate non-empty `event_key`/`summary`/`href` and parseable category/tone; return the number inserted. Tests: duplicate key inserts once, missing agent is skipped, existing row is never updated.
- [ ] T011 Implement `list_notifications` and `notification_counts` in `src/storage/sqlite/notification.rs`. Constraints: only `agent_id IN (visible whitelist)` and `created_at >= now - 30 days`; `LEFT JOIN notification_read` for this `user_id`; filter `all|unread|read`; keyset `(created_at, id) < cursor` ordered `created_at DESC, id DESC`; `limit` is the caller's page size; `read` is `read_at IS NOT NULL`; counts are `all`, `unread`, `read` independent of the selected filter. Tests: empty whitelist, filter exclusivity, counts, tied-timestamp paging, retention boundary.
- [ ] T012 [P] Add `src/notifications/cursor.rs`: URL-safe base64 (no padding) of `{"at":<millis>,"id":<i64>}`; encode/decode and tests for a round trip, a malformed cursor, and a missing field. The API returns `400` for a malformed cursor.
- [ ] T013 Add `src/notifications/mod.rs` with `Notifications` (holds the storage handle), add `pub notifications: Notifications` to `VizierDependencies` and construct it after the storage in `src/dependencies.rs`, and implement `record(&NotificationDraft)`: verify the agent still exists, insert best-effort, `tracing::warn!` on failure, and never propagate the error to the source. Add a unit test with a missing agent and a duplicate event key. Do not add source methods yet.
- [ ] T014 Add the notification HTTP surface in `src/channels/http/api/v1/notifications.rs`: `GET /notifications` (filter, before, limit, items+counts+next) and `GET /notifications/count` (counts). Resolve `visible_agents` before every read, clamp `limit` 1–50, decode the cursor, and return 400 for an invalid filter/cursor. Register `/notifications` under `require_auth` in `src/channels/http/api/v1/mod.rs`; add the schemas/paths to `ApiDoc` in `src/channels/http/mod.rs`. Add handler tests for 401, empty visibility, counts and paging.
- [ ] T015 [P] Add `webui/app/api/endpoints/notifications.ts` with `NotificationFilter`, `NotificationItem`, `NotificationCounts`, `NotificationPage`, `listNotifications`, `notificationCounts`, `setNotificationRead` and `markAllNotificationsRead` calls, shaped like the existing endpoint modules.
- [ ] T016 Add `webui/app/stores/notifications.ts` shell: `open`, `filter`, `items`, `counts`, `next`, `loading`, `error`, `loaded`; `openPanel`, `closePanel`, `load`, `loadMore`, `reset`; use the API client from T015. No polling or read actions yet.

**Checkpoint**: the database, storage, read API, API client and store shell exist; a test page can load an empty inbox.

---

## Phase 3: User Story 1 — See what happened and open it from any screen (P1) 🎯 MVP

**Goal**: a Notifications entry above Settings with an unread badge; a slide-over listing the latest notifications newest first; rows show agent/what/when and open their destination; paging; loading/empty; collapsed rail.

**Independent Test**: with an agent that has produced a notable event, sign in, open a non-Home screen, confirm the entry/badge, open the panel, load older, select a row, and land on its destination. Repeat with an empty inbox and the collapsed rail.

- [ ] T017 [US1] Implement the store's first-page load, `loadMore` and panel open/close in `webui/app/stores/notifications.ts` (use T015; keep the API's `next`), with a unit test in `webui/app/stores/notifications.test.ts`.
- [ ] T018 [P] [US1] Create `webui/app/screens/notifications/NotificationRow.tsx`: fixed unread gutter (6–8px, empty for read), `Sigil size="sm"`, agent name from `useAgents` with an id fallback, `feedTime` meta, summary with `text-muted`/`text-failed-text`, a visually hidden `Unread`, and `href` navigation. Leave the mark action to US2 via the `ListItem` `action` prop.
- [ ] T019 [US1] Create `webui/app/screens/notifications/NotificationsPanel.tsx`: `SlideOver` with title `Notifications`, header, list of T018 rows, `LoadMore`, and the All-only view. No filters or setup section yet.
- [ ] T020 [US1] Wire the panel and entry in `webui/app/layouts/AppShell.tsx`: render `<NotificationsPanel />` in both desktop and mobile branches; pass `notifications={{ unread: counts.unread, busy: false, onOpen }}` to `Sidebar`; close the phone nav drawer before opening the panel; call `useNotifications.openPanel()`. This uses the design `Sidebar`/`NavItem` from T002/T003.
- [ ] T021 [US1] Add `Skeleton` loading, `EmptyState` for no notifications (`bell`, “Nothing yet”), a basic error with Retry (keep the last counts, no false zero), and `LoadMore` in `NotificationsPanel.tsx`; the full per-filter empty states are US2.
- [ ] T022 [US1] Add tests: reuse T014's list/count handler coverage; add the WebUI store test in `webui/app/stores/notifications.test.ts` and the first `webui/e2e/notifications.spec.ts` case (sidebar entry/badge, open, row destination, empty state, phone/collapsed entry basic).
- [ ] T023 [US1] Run `cargo test` and `cd webui && npm test && npm run typecheck`; manually verify the panel on desktop and in the collapsed rail; record the US1 checkpoint in `specs/029-webui-notifications/quickstart.md`.

**Checkpoint**: US1 is functional without read state or filters; the inbox and navigation are independently testable.

---

## Phase 4: User Story 2 — Separate read from unread and clear the inbox (P1)

**Goal**: All/Unread/Read filters with counts, per-row read/unread, “Mark all read”, durable per-person read state, and specific empty states.

**Independent Test**: with several notifications, mark one read, one unread, then all read; reload, open a second tab/device session, and verify counts and filters. Open the panel without selecting and confirm nothing becomes read.

- [ ] T024 [US2] Add `set_notification_read` and `mark_all_notifications_read` to the trait in `src/storage/notification.rs` and the `VizierStorage` forwarding in `src/storage/mod.rs`; implement both in `src/storage/sqlite/notification.rs`. Constraints: unread = no read row; read = insert/upsert `(user_id, notification_id, read_at)`; unread = delete; mark-all = `INSERT OR IGNORE … SELECT` over the visible, retained unread set; per-id returns false for missing/invisible; 404 at the API. Tests: idempotent read/unread, mark-all beyond the page, a concurrently inserted row stays unread, non-visible id returns false.
- [ ] T025 [US2] Add `PUT /notifications/{id}/read` (`{read: bool}`) and `POST /notifications/read-all` to `src/channels/http/api/v1/notifications.rs` (after T024's storage methods); return `NotificationCounts`; 400 for a missing/non-boolean body, 404 for missing/invisible; update `ApiDoc`. Add handler tests.
- [ ] T026 [US2] Add `filter`, `counts`, `markRead` and `markAll` to `webui/app/stores/notifications.ts`: optimistic row/count updates, server counts on response, remove rows that leave the Unread/Read filter, and restore on failure; add store tests.
- [ ] T027 [US2] Add `FilterTabs` (All/Unread/Read with counts, default All), a `Mark all read` action (disabled at unread 0), and the row `action` (`ListItem` slot) to `NotificationsPanel.tsx`/`NotificationRow.tsx`: unread → `check`/“Mark as read”, read → `rotate-ccw`/“Mark as unread”; hover/focus on desktop, always visible on touch; the action must not navigate.
- [ ] T028 [US2] Make row selection mark read and navigate in `NotificationRow.tsx`/`NotificationsPanel.tsx`; preserve the selected filter across close/reopen and read changes; apply filter membership (read disappears from Unread, unread disappears from Read) with tests.
- [ ] T029 [US2] Add the per-filter empty states to `NotificationsPanel.tsx`: All (`bell`, “Nothing yet”), Unread (`circle-check`, “You’re all caught up.”), Read (`history`, “Nothing read yet.”), plus the existing error state; keep the exact messages in `contracts/notifications-ui.md`.
- [ ] T030 [US2] Add tests: the Rust storage read/unread/mark-all cases in `src/storage/sqlite/notification.rs` (reuse T025's handler coverage); store tests for optimistic/filter membership in `webui/app/stores/notifications.test.ts`; Playwright cases for filters, mark read/unread, mark all, reload persistence and the open-without-reading rule.
- [ ] T031 [US2] Run `cargo test` and `cd webui && npm test && npm run typecheck`; verify read state across a reload and a fresh login; record the US2 checkpoint in `specs/029-webui-notifications/quickstart.md`.

**Checkpoint**: the inbox has a complete read loop; US1/US2 work independently of the source hooks.

---

## Phase 5: User Story 3 — Everything the old panel showed is still accounted for (P1)

**Goal**: notifications for task runs, dream cycles, background jobs, integration changes, channel reply hours and agent start failures; Home's away panel and the old feed are gone.

**Independent Test**: trigger each of the six categories; confirm one notification per completed event with the expected text/tone/href; leave a task/job running and confirm no notification; open Home and confirm no away panel. API tests can cover a category without the WebUI; the panel display builds on US1.

- [ ] T032 [US3] Implement `src/notifications/text.rs` for the US3 categories, matching `contracts/notification-sources.md`: task answered/no-answer/interrupted; dream singular/plural; job kind/outcome/reason; integration actor/action/family page; grant on/off; channel reply platform/count; start failure with retry seconds. Add unit tests for every branch, quotes and hrefs.
- [ ] T033 [US3] Add the source methods to `src/notifications/mod.rs`: `task_run_ended`, `dream_ended`, `job_ended`, `integration_events`/`integration_grant`, `agent_start_failed`; build the deterministic `event_key`s and call `record` (best-effort). Tests for event keys and one row per key.
- [ ] T034 [P] [US3] Add task-run notifications in `src/scheduler/mod.rs`: capture `agent_id`/slug/title at fire time; notify after a successful `close_task_run` in `fire`, `recover` (title lookup with slug fallback), and the dispatch-failure path; non-answered and interrupted are `failed`. The startup sweep is wired in T040. Tests: answered/no-response/interrupted, deleted-task title fallback, one per run.
- [ ] T035 [P] [US3] Add the dream notification in `src/scheduler/dream/mod.rs`: after consolidation, count the cycle's journal entries with `list_dream_entries_by_cycle`, skip zero, and notify with `Dreamed: …`; test the count/wording and the zero-skip.
- [ ] T036 [P] [US3] Add background-job notifications in `src/agents/background/mod.rs`: change `BackgroundJobs::new` to take and store `Notifications` (the `dependencies.rs` construction is wired in T040); notify on `Reported` (run and recovery), `Cancelled` (only when the guarded transition won), and `Undelivered`; never on `Reporting`; tests for terminal-only and one per job. The startup `Interrupted` sweep is wired in T040.
- [ ] T037 [P] [US3] Add integration/grant notifications in `src/integrations/service.rs` after each committed write: one per event × affected agent (legacy rows), resolve the actor name once (`Requester::User`/`Requester::Agent`, fallback id), never include setting values; test create/edit/enable/disable/delete/unlink/grant.
- [ ] T038 [P] [US3] Add the start-failure notification in `src/agents/supervisor.rs`: track a failure-episode id opened at the first failure after a successful start, reuse it across retries, clear it on a successful start (so a later failure is a new episode), notify once with the error/retry seconds, and skip a delete/shutdown stop. Tests: one per episode, retry silence, recovery then new failure, delete silence.
- [ ] T039 [US3] Add the channel-reply projection: `ReplyBatch` and `channel_reply_batches(from_hour,to_hour)` to the trait in `src/storage/notification.rs` with forwarding, implemented in `src/storage/sqlite/notification.rs` (global, `Response` rows in `discord__`/`telegram__`, `GROUP BY agent, channel, hour`, max timestamp); `project_replies(now)` in `src/notifications/mod.rs` using the `notifications.replies_through` state key; and call it at the start of the list/count/read-all handlers (`src/channels/http/api/v1/notifications.rs`). Rules: first activation sets the watermark to the last closed hour and imports nothing; later projects closed hours only, caps at 30 days, inserts idempotently, then saves the watermark. Tests: first activation, one per conversation-hour, current hour skipped, no duplicates, catch-up cap.
- [ ] T040 [US3] Wire `src/dependencies.rs`: pass the existing `Notifications` to `BackgroundJobs`; make the startup task-run/job sweeps return the rows they interrupted; record those notifications after the service exists; keep startup best-effort/logged. Test the startup sweep path where practical.
- [ ] T041 [P] [US3] Remove Home's away panel: delete `webui/app/screens/home/AwayFeed.tsx` and remove its import plus the 340px right column/aside from `webui/app/routes/home.tsx`; Home keeps the greeting, activity panel and court and uses the width.
- [ ] T042 [P] [US3] Remove the old feed backend: delete `src/channels/http/api/v1/overview/feed.rs`, `src/storage/feed.rs` and `src/storage/sqlite/feed.rs`; remove the `pub mod feed;`/`mod feed;` declarations and `FeedStorage` from `src/storage/mod.rs` and `src/storage/sqlite/mod.rs`; remove the `feed` module/route from `src/channels/http/api/v1/overview/mod.rs`; remove the feed OpenAPI paths/schemas from `src/channels/http/mod.rs`. The channel-reply query now lives in notification storage.
- [ ] T043 [US3] Remove the old feed WebUI client: remove `FeedEvent`, `Feed` and `getFeed` from `webui/app/api/endpoints/overview.ts`; move `feedTime` (and its test) from `webui/app/lib/overview.ts`/`overview.test.ts` to `webui/app/lib/format.ts`/`format.test.ts`; update the notification row import if US1 already created it (coordinate with T018).
- [ ] T044 [US3] Add the cross-cutting US3 tests and e2e steps: the per-source/text unit tests are in T032–T039; add a category/API integration test and `webui/e2e/notifications.spec.ts` assertions that Home has no `Since you were away` and the panel shows an integration event; extend `quickstart.md` §1–§4 and §7 with the integration/task/background/reply checks. Mark the dream check as live-only (dummyplug writes no journal entries).

**Checkpoint**: all six old categories are in the inbox, no away panel remains, and the old feed code/API is gone. US3 is testable via API and the panel.

---

## Phase 6: User Story 4 — Watch setup and downloads from the notification panel (P2)

**Goal**: live setup progress above the filters (starting/downloading, bytes/percent/ETA, per-artifact), a separate nav activity indicator, durable ready/failure outcomes, and the chat's detailed card replaced by a compact line.

**Independent Test**: start a setup with a download, leave the chat, confirm the activity indicator and live card; confirm a ready notification when it downloads (or recovers from a failure); confirm a download-less quick restart adds no ready; make the setup fail and confirm one failure notification and the chat's failure banner.

- [ ] T045 [US4] Extend `src/setup.rs`: add `name` to the setup entry/`SetupState` (from `config.name` at `build_started`, with a fallback for a download-created entry), add a monotonic per-agent `download_epoch`/`downloaded` result reset on a new top-level build and advanced by `download_started`, and keep overlapping builds from wiping it. Tests for the epoch/reset/overlap.
- [ ] T046 [US4] Extend `BuiltAgent::build`/`build_quietly` in `src/agents/supervisor.rs` to carry `build_id` and `setup_downloaded` (snapshot the setup epoch before/after); pass them through `prebuilt`; tests for build/download flag propagation.
- [ ] T047 [US4] Add the ready outcome in `src/agents/supervisor.rs`: after recording Running, if `setup_downloaded` or the build is recovering from a failure episode, record one `agent_setup_ready` (`{name} is ready.`, `success`, chat href); no ready for a download-less, failure-less quick restart; one per build/episode. Add the ready wording to `src/notifications/text.rs`; tests for download, recovery, quick restart and no duplicate. Depends on T038.
- [ ] T048 [P] [US4] Create `webui/app/screens/notifications/SetupActivity.tsx`: map `SetupState.downloads` (kind/label/percent/detail/status/error) and `setupProgress`/`eta` into the `AgentStartingCard` downloads slot; full width; `role="status"`; indeterminate when total is 0; a link to the agent chat; no read state. Add `name` to the `SetupState` type in `webui/app/api/endpoints/agents.ts` if T045 has not already.
- [ ] T049 [US4] Add the live setup section to `NotificationsPanel.tsx` above the sticky filters: filter `useAgents.setups` to `starting|downloading`, stack cards for multiple agents, use `SetupState.name` with store/id fallback, hide when none; it stays visible regardless of All/Unread/Read.
- [ ] T050 [P] [US4] Update `webui/app/stores/agents.ts` to poll `/utils/setup-state` every 1s while the tab is visible (not only when an agent already reports starting), so the activity indicator appears within 2s; derive `busy` as any visible setup in `starting|downloading` for `AppShell`/`Sidebar`.
- [ ] T051 [P] [US4] Update `webui/app/screens/chat/AgentStateBanner.tsx`: replace the detailed `AgentStartingCard` with a compact line (`Starting {name} · {step} · {percent}% · Messages wait in line · View in Notifications`), open the panel on the link, and keep the failed banner with Retry now/Open config plus the connection banners. Remove the detailed card from chat only.
- [ ] T052 [US4] Add tests and quickstart: Rust setup epoch/ready/recovery/quick-restart/no-duplicate tests; WebUI tests for the setup section/download mapping/poll trigger; `quickstart.md` §5–§6 with the live Ollama download/ready/quick-restart steps and the failure/no-duplicate checks marked live where required.
- [ ] T053 [US4] Run `cargo test` and `cd webui && npm test && npm run typecheck`; manually verify the live setup card and the chat compact line; record the US4 checkpoint in `specs/029-webui-notifications/quickstart.md`.

**Checkpoint**: setup progress is live-only, ready/failure outcomes are durable, the chat card is gone, and the setup activity indicator is separate from unread.

---

## Phase 7: User Story 5 — See only what belongs to you and what you may see (P2)

**Goal**: only visible agents' notifications; per-person read state; revocation removes rows/count immediately; agent deletion removes notifications/read rows; 30-day retention.

**Independent Test**: two people with different agent access see only their agents' rows and their own read state; revoke access and reload; delete an agent and check the tables; an old notification is neither listed/counted and is purged.

- [ ] T054 [US5] Delete an agent's notifications in `src/storage/sqlite/agent.rs::delete_agent` (next to the other agent-owned tables; `notification_read` cascades); test that other agents' rows/read states survive.
- [ ] T055 [US5] Add `purge_notifications_before` to the trait in `src/storage/notification.rs` with forwarding and implement it in `src/storage/sqlite/notification.rs`; add `Notifications::purge_before` in `src/notifications/mod.rs`; run it at startup in `src/dependencies.rs` and once a day from the scheduler's minute tick in `src/scheduler/mod.rs`. List/count already filter `created_at >= now - 30 days`; test the boundary and cascade.
- [ ] T056 [US5] Add the startup orphan cleanup for notifications whose `agent_id` no longer exists in `agent_config` (in `src/dependencies.rs` or notification storage), so a delete/insert race cannot leave a row; test it.
- [ ] T057 [US5] Add visibility/API tests in `src/storage/sqlite/notification.rs` and `src/channels/http/api/v1/notifications.rs`: only the whitelist, empty whitelist returns empty/zero, read/mark-all on an invisible id is 404, counts match the visible set.
- [ ] T058 [US5] Add retention and read-independence tests: rows inside/outside 30 days, purge removes old rows/read rows, two users have independent read states for the same notification, and the unread counts are per person.
- [ ] T059 [US5] Run `cargo test` and `cd webui && npm test && npm run typecheck`; manually verify a second user, revocation and agent deletion; record the US5 checkpoint in `specs/029-webui-notifications/quickstart.md`.

**Checkpoint**: access is checked at every read, deletion cleans up, and retention is enforced.

---

## Phase 8: User Story 6 — Stay current while the app is open (P2)

**Goal**: the badge updates within 30s, an open list receives new items at the top, read changes converge across tabs, and load failures show an error/retry without a false zero.

**Independent Test**: keep the app open; create a notification; observe the badge/list within 30s; mark it read in a second tab; both converge; simulate a failed list/count request and confirm the error/retry and last counts.

- [ ] T060 [US6] Add polling to `webui/app/stores/notifications.ts`: `start()` polls `GET /notifications/count` every 15s while visible, refreshes the open list on the same cadence, and returns a stop; `reset()` on sign-out/mount. Keep the selected filter and scroll.
- [ ] T061 [US6] Call `useNotifications.start()` from `webui/app/layouts/AppShell.tsx` with cleanup, next to `useAgents.start()` (after T060); do not add a WebSocket/SSE.
- [ ] T062 [P] [US6] Add `webui/app/lib/notifications.ts` helpers for a first-page merge (new items at top by id, no duplicates, preserve older pages, update read flags/counts) and unit tests in `webui/app/lib/notifications.test.ts`.
- [ ] T063 [US6] Add the panel/store error path: a failed list shows `EmptyState kind="error"` with Retry, keeps the selected filter and the last known counts (no false zero), and recovers on the next successful poll.
- [ ] T064 [US6] Handle an arriving notification under the current filter in `NotificationsPanel.tsx`: appear at the top when it matches, do not appear under Read when unread, preserve the filter and avoid an unexpected scroll jump; test with the store/merge helper.
- [ ] T065 [US6] Add a Playwright two-page test in `webui/e2e/notifications.spec.ts`: create a notification, see the badge/list update without reload; mark read in page B and see page A converge; test the error/retry state where feasible.
- [ ] T066 [US6] Run `cargo test` and `cd webui && npm test && npm run typecheck`; manually verify the 15s poll and cross-tab convergence; record the US6 checkpoint in `specs/029-webui-notifications/quickstart.md`.

**Checkpoint**: freshness, cross-tab read state and failure handling meet the 30s budget without a new transport.

---

## Phase 9: User Story 7 — Use it on a phone and with assistive technology (P3)

**Goal**: the drawer entry opens a full-screen sheet at 390px, with wrapping filters, safe areas, always-visible actions, focus containment/return, escape/back, and screen-reader names.

**Independent Test**: complete the main journeys at a phone viewport and with keyboard + screen reader; confirm no horizontal scroll, the setup card is full width, Escape/back closes, focus returns, and failure is not colour-only.

- [ ] T067 [US7] Make `NotificationsPanel`/`SlideOver` full-screen at phone widths, render the panel in the phone branch of `AppShell`, and close the nav drawer before opening it; ensure back/Escape closes and returns to the previous screen.
- [ ] T068 [US7] Add the phone layout polish in `NotificationsPanel.tsx`/`NotificationRow.tsx`: wrapping filters, Mark all read on its own row when needed, always-visible row actions (no hover), full-width setup cards, bottom safe-area padding, and no horizontal scroll at 390px.
- [ ] T069 [US7] Add the accessibility contract: the entry's accessible name includes unread/busy, the dialog is named `Notifications`, filters are a named group with pressed state, the action has `Mark as read`/`Mark as unread`, focus is contained while open and restored to the entry, and the error is `role="alert"` with Retry.
- [ ] T070 [P] [US7] Add the row semantics: the unread dot is decorative and the row includes a visually hidden `Unread`, the subtitle is readable at 2 lines on a phone, and failure is conveyed by wording plus colour.
- [ ] T071 [P] [US7] Add the setup accessibility: the card is `role="status"`, the progress bar has `aria-valuemin/max/now` or is indeterminate with a label, and download/error lines are readable.
- [ ] T072 [US7] Add the Playwright phone cases in `webui/e2e/notifications.spec.ts`: drawer → Notifications → full-screen dialog; filters/actions; close/back/focus; no horizontal overflow; full-width setup card when a setup is present.
- [ ] T073 [US7] Extend `webui/e2e/a11y.spec.ts` (or the notifications spec with AxeBuilder) to open the panel/filters/setup and assert no serious/critical violations; verify keyboard/Escape/focus manually.
- [ ] T074 [US7] Run `cargo test`, `cd webui && npm test && npm run typecheck`, and the focused Playwright specs; record the US7 checkpoint in `specs/029-webui-notifications/quickstart.md`.

**Checkpoint**: the inbox is usable at 390px and with assistive technology; no horizontal overflow and no colour-only state.

---

## Phase 10: Polish & cross-cutting concerns

**Purpose**: docs, API artifacts, old-feed cleanup, full validation and the three-submodule release order.

- [ ] T075 [P] Update `docs/src/api-integration/rest-api.md`: remove the `/overview/feed` row and visit paragraph, retitle the Home section, and add the Notifications section (list, count, read, read-all, filters/cursor/counts/visibility, setup-state name).
- [ ] T076 [P] Update `CLAUDE.md` with the notification storage/creation/read-state model, the setup live/outcome split, the panel wiring and the removed feed.
- [ ] T077 [P] Add a #82 checklist entry for the now-unused visit tracking (`record_visit`, `user.last_seen_at`, `user.previous_visit_at`) if it is not removed here; do not drop columns in this feature.
- [ ] T078 Regenerate the OpenAPI/WebUI types with `scripts/openapi.mjs` (or the project's generator) after the notification endpoints, `SetupState.name` and the feed removal; commit `webui/openapi.json` and `webui/app/api/schema.d.ts`.
- [ ] T079 Run the old-feed removal check: `grep -rn "overview/feed\|Since you were away\|AwayFeed\|FeedStorage\|getFeed" src webui docs` must be empty except historical specs; remove any residue and unused imports.
- [ ] T080 Walk `quickstart.md` end to end on a scratch workspace: §1 integration, §2 read/filter/persistence, §3 dummyplug task, §4 dummyplug background job, §5 start failure, §7 SQLite reply fixture, §9 visibility/retention/delete, §10 Home/sidebar/phone. Record actual outputs.
- [ ] T081 Run the marked live checks: §6 Ollama download → live setup → ready → quick restart no ready → failure/no duplicate; §8 real-provider dream → dream notification. Mark them as live in the record.
- [ ] T082 Run `cargo fmt --check`, `cargo clippy --all-targets`, `cargo test`.
- [ ] T083 Run `cd webui && npm test && npm run typecheck && npm run e2e`.
- [ ] T084 Commit in cascade order: `design/` submodule (T001–T006), `webui/` submodule (the panel/client/store/home/chat work), then the vizier repo root (backend/docs/OpenAPI) and bump both gitlinks; flag the `/overview/feed` removal as a breaking API change in the conventional commit/changelog.
- [ ] T085 Final review in `specs/029-webui-notifications/tasks.md` and `quickstart.md`: confirm no push/service worker work, no per-user notification fan-out, no model call for notification text, no secrets/settings values in summaries, the setup live section has no read state, and every checkpoint passes.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup & design)**: starts immediately; T006 depends on T001–T005.
- **Phase 2 (Foundational)**: T007 first; T008/T009/T012 can ride with it; T010/T011 depend on T007–T009; T013 depends on T010/T011; T014 depends on T011/T013; T015 is independent of Rust; T016 depends on T015. **Blocks all user stories.**
- **Phase 3 (US1)**: after Phase 2; T017/T018 can start together; T019 depends on both; T020 depends on T019 + design T002/T003.
- **Phase 4 (US2)**: backend T024/T025 can start after Phase 2; T026/T027 need US1's store/panel; T028/T029 follow T027.
- **Phase 5 (US3)**: starts after Phase 2 and T013; T032 → T033; T034–T038 are parallel call sites after T033; T039 after T033; T040 after T033/T036/T038; T041–T043 are independent removals; T044 after the rest.
- **Phase 6 (US4)**: after US1 (panel) and US3 T038 (failure episode); T045 → T046 → T047; T048/T050/T051 can parallel after the relevant primitives; T049 after T048/US1 panel.
- **Phase 7 (US5)**: storage/API tests can start after Phase 2 + US2; T054 is independent, while T055 and T056 both touch `dependencies.rs` and run in sequence; T057/T058 after their implementations.
- **Phase 8 (US6)**: after US1 + US2; T060 → T061/T062/T063/T064; T065 after the UI; T066 last.
- **Phase 9 (US7)**: after US1 + US2 + US4; T067 → T068/T069/T070/T071; T072/T073 after the UI.
- **Phase 10 (Polish)**: after all desired stories.

### Dependency Graph

```text
Phase 1 (design primitives)
        │
Phase 2 (schema/storage/API/store)
        ├── US1 ──► US2 ──► US6
        │             └────► US7
        ├── US3 ──► US4 ──► US7
        └── US5  (lifecycle/tests; read-state tests use US2)
                            │
                     Phase 10 (Polish)
```

### User Story Dependencies

- **US1 (P1)**: Foundation only. No dependency on other stories.
- **US2 (P1)**: Backend can start after Foundation; UI depends on US1's store/panel.
- **US3 (P1)**: Foundation + `Notifications` service; independent of US1/US2 for API tests, displayed by US1.
- **US4 (P2)**: US1 for the panel; US3 T038 for failure dedupe; backend can parallel with US1.
- **US5 (P2)**: Foundation + US2 for read-state tests; lifecycle work is independent.
- **US6 (P2)**: US1 + US2.
- **US7 (P3)**: US1 + US2 + US4.

### Within Each User Story

- Tests before the code they cover where practical; Rust tests in the named source file.
- Models/schema before services; services before endpoints; endpoints before UI; core before integration.
- Complete a story's checkpoint before moving to the next priority, unless staffed in parallel.

### Parallel Opportunities

- T001/T002/T004/T005 can start together; T003 follows T002.
- T008/T012 can start with T007; T009 follows T007; T015 then T016.
- T034–T038 are five source call sites after T033; T041/T042 are independent removals; T043 follows T018.
- T045 → T046 → T047; T048/T050/T051 can run in parallel afterwards.
- T054 is independent; T055/T056 share `dependencies.rs`, so sequence them; T057/T058 are test files.
- T060 → T061; T062 can run in parallel with them.
- T067 first; T068/T069 follow; T070/T071 are separate files and can run in parallel.
- T075/T076/T077 (docs/CLAUDE/#82) are independent.

---

## Parallel Examples

### User Story 1

```bash
# After the store shell exists:
Task: "Implement the first-page load/loadMore/open in webui/app/stores/notifications.ts"
Task: "Create NotificationRow.tsx"

# Then, after both:
Task: "Create NotificationsPanel.tsx"
Task: "Wire the panel/entry in AppShell.tsx"
```

### User Story 2

```bash
# Backend and API can start together:
Task: "Implement read/unread/mark-all in src/storage/sqlite/notification.rs"
Task: "Add PUT /{id}/read and POST /read-all in src/channels/http/api/v1/notifications.rs"

# Then the store and UI:
Task: "Add filters/read actions to webui/app/stores/notifications.ts"
Task: "Add FilterTabs, mark-all and the row action to the panel"
```

### User Story 3

```bash
# After T032/T033 (wording + service methods):
Task: "Add task-run notifications in src/scheduler/mod.rs"
Task: "Add the dream notification in src/scheduler/dream/mod.rs"
Task: "Add background-job notifications in src/agents/background/mod.rs"
Task: "Add integration/grant notifications in src/integrations/service.rs"
Task: "Add the start-failure notification in src/agents/supervisor.rs"

# Independent removals:
Task: "Remove the away panel from webui/app/routes/home.tsx and delete AwayFeed.tsx"
Task: "Remove the feed backend files/routes"
Task: "Remove the WebUI feed client and move feedTime"
```

### User Story 4

```bash
Task: "Extend src/setup.rs with name and download epoch"
Task: "Create webui/app/screens/notifications/SetupActivity.tsx"
Task: "Update webui/app/stores/agents.ts poll cadence"
Task: "Replace the chat starting card with the compact line"
```

### User Story 5

```bash
Task: "Delete an agent's notifications in src/storage/sqlite/agent.rs"
Task: "Add retention purge + startup/daily calls"
Task: "Add the startup orphan cleanup in src/dependencies.rs"
Task: "Add visibility/retention/read-independence tests"
```

### User Story 6

```bash
Task: "Add count/list polling to webui/app/stores/notifications.ts"
Task: "Start the poll from webui/app/layouts/AppShell.tsx"
Task: "Add the merge helper + tests in webui/app/lib/notifications.ts"
Task: "Add the error/retry and arriving-item behavior in the panel"
```

### User Story 7

```bash
Task: "Make the panel full-screen and wire the phone drawer in AppShell"
Task: "Add the focus/accessible-name contract"
Task: "Add the row unread/failure semantics"
Task: "Add the setup progressbar/status semantics"
Task: "Add the Playwright phone and axe checks"
```

---

## Implementation Strategy

### MVP First (US1 + US2)

1. Phase 1 (design primitives) and Phase 2 (foundation).
2. US1: the sidebar entry, badge, panel, rows, paging and destination.
3. US2: filters, read/unread, mark all and per-person persistence.
4. **STOP and VALIDATE**: run the inbox/read-loop Playwright and API checks. This is the smallest useful notification system (the original request's “list + read/unread filter”).
5. US3 completes P1 coverage (the six old categories + Home removal); US4–US7 follow in priority order.

### Incremental Delivery

- Foundation → US1 → US2 → US3: the P1 inbox is complete.
- US4 adds setup live/ready/failure; US5 adds lifecycle/retention; US6 adds freshness/cross-tab; US7 adds phone/accessibility.
- Each story's checkpoint is independently testable; no story requires a later story to work, except the noted UI dependencies.

### Parallel Team Strategy

After Phase 2:
- Developer A: US1 then US2 (UI foundation).
- Developer B: US3 source hooks + Home/feed removal.
- Developer C: US4 setup + US5 lifecycle, then US6/US7 once the UI lands.
- The design submodule must be committed before the WebUI work; the root backend can proceed independently.

---

## Notes

- `[P]` tasks are different files with no incomplete dependency.
- Include tests with each story; the Rust tests live in the source file's `#[cfg(test)]` module, WebUI unit tests in `*.test.ts`, Playwright in `webui/e2e/`.
- Do not add browser push, a service worker, VAPID keys or push subscriptions; R14 defers them.
- Do not create per-user notification rows; read state is the only per-person row.
- A notification write is best-effort and never fails the source event.
- The old `/overview/feed` removal is a deliberate breaking HTTP/WebUI change; call it out in the commit/changelog.
- Commit after each logical task group; the final order is design → webui → vizier with gitlink bumps.
