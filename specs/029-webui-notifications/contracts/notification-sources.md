# Contract: Notification Sources

**Feature**: `029-webui-notifications` | **Plan**: [../plan.md](../plan.md) | **Data model**: [../data-model.md](../data-model.md)

Every notification is created after the source's transition is durable. The one exception is channel replies, which are a read-time projection over `session_history`. Wording lives in `src/notifications/text.rs`; no handler builds a sentence.

## Source matrix

| Source | Trigger | `event_key` | `tone` | Destination |
|---|---|---|---|---|
| Task run | `close_task_run` succeeded | `task_run:{run_id}` | answered `default`; no-answer/interrupted `failed` | `/{agent}/tasks?task={slug}&run={run_id}` |
| Dream cycle | Consolidation ends, entries > 0 | `dream:{cycle_id}` | `default` | `/{agent}/dream?cycle={cycle_id}` |
| Background job | Terminal transition | `job:{job_id}` | reported/cancelled `default`; undelivered/lost `failed` | `/{agent}/chat` |
| Integration event | `integrations::service` write committed | `integration:{integration_id}:{action}:{at_ms}:{ordinal}:{agent_id}` | `default` | `/{agent}/integrations/{channels\|addons\|mcp}` |
| Integration grant | `set_grant` changed | `grant:{agent_id}:{family}:{on\|off}:{at_ms}` | `default` | `/{agent}/integrations/{family}` |
| Channel replies | Closed-hour projection | `reply:{agent_id}:{channel}:{hour}` | `default` | `/{agent}/chat?channel={channel}` |
| WebUI chat reply | A WebUI turn produced a reply and was saved | `chat:{history_uid}` | `default` | `/{agent}/chat/{topic}` |
| Agent setup ready | Supervisor starts a build that downloaded ≥ 1 artifact | `setup_ready:{agent_id}:{build_id}` | `success` | `/{agent}/chat` |
| Agent start failure | Supervisor enters a new failure episode | `start_failed:{agent_id}:{episode_id}` | `failed` | `/{agent}/config` |

All rows carry the agent id; the row title in the UI is the agent's display name from the client agent store (or the id). The summary never repeats model output.

## Task runs

- **When**: after `TaskRunStorage::close_task_run` returns success, in:
  - the spawned close in `VizierScheduler::fire` (capture `task.title`, `task.agent_id`, `task.slug` before spawning);
  - `VizierScheduler::recover` for a resumed run (look up the title; fall back to the slug);
  - the startup sweep for runs left open by a previous process (the sweep returns the swept rows; the service records them after the notification service is constructed);
  - the dispatch-failure path (`NoResponse`).
- **Text**: `Task “{title}” answered.` / `Task “{title}” ended without an answer.` / `Task “{title}” was interrupted.`
- **Tone**: answered `default`; no-response and interrupted `failed`.
- **Href**: the old feed's task-run deep link.
- **Missing task**: the title falls back to the slug; the notification remains after the task/run is deleted. The destination may show a not-found state.
- **No dispatch**: a run that never reached the agent still ends `NoResponse` and notifies; the run is visible as an event rather than silently absent.

## Dream cycles

- **When**: in `DreamScheduler`'s spawned cycle, after consolidation and as the status returns to Idle. Count the cycle's journal entries with `list_dream_entries_by_cycle`; skip when the count is 0.
- **Text**: `Dreamed: 1 journal entry.` / `Dreamed: {n} journal entries.`
- **Tone**: `default`.
- **Href**: `/{agent}/dream?cycle={cycle_id}`.
- **No model call**: the count reads storage; it never asks the model. A dummyplug dream writes no entries, so it produces no notification (the quickstart marks the live/provider check).

## Background jobs

- **When**: after a terminal transition in `BackgroundJobs`:
  - `Reporting → Reported` in `run_job` (delivery succeeded);
  - `Reporting → Undelivered` in `deliver`;
  - `Running → Cancelled` in `cancel` (only when the guarded transition won);
  - `Reporting → Reported` in `recover` (a recovered reporting job);
  - `running/reporting → Interrupted` in the startup sweep (the sweep returns the jobs; the service records them).
- **Never** on `Running → Reporting`.
- **Text**: the old feed's wording:
  - `Background batch reported back.` / `Delegation reported back.`
  - `Background batch was cancelled (reason).` / `Delegation was cancelled (reason).`
  - `… finished, but its report could not be delivered.` (with the delivery reason when present)
  - `… was lost when vizier restarted.`
- **Tone**: reported/cancelled `default`; undelivered/interrupted `failed`.
- **Href**: `/{origin_agent}/chat`.
- **Dedupe**: the terminal transition is already exactly-once; the unique `job:{id}` key makes recovery/retry safe.

## Integration changes and grants

- **When**: `integrations/service.rs` after `save_integration`, `delete_integration`, `unlink` or a changed `set_grant` has committed. One notification per event and affected agent (a legacy integration linked to several agents produces one per agent).
- **Actor**: resolve `Requester::User` to the username (`UserStorage::get_user_by_id`) or `Requester::Agent` to the agent name (`AgentStorage::get_agent`); fall back to the id. Resolve once per write.
- **Text**: the old feed's `{who} {changed} {what}.` for created/edited/enabled/disabled/deleted/unlinked; grants use `{who} let the agent manage its own {name} integrations.` / `{who} stopped the agent managing its own {name} integrations.`
- **Href**: the family page (`channels`, `addons`, `mcp`).
- **Never** include a setting value, token or secret; only the integration name, action and actor.
- **Key**: `integration:{integration_id}:{action}:{at_ms}:{ordinal}:{agent_id}`; grants use `grant:{agent_id}:{family}:{on|off}:{at_ms}`. A retried HTTP write is a new write (the service itself is not idempotent), so its notification is also new; this matches the existing change record.
- **Failure**: a notification failure logs and does not fail the committed integration write.

## Channel replies

- **When**: `Notifications::project_replies(now)` before a list/count/read-all request. It projects closed hours after `notifications.replies_through`; the first activation starts at the last closed hour and imports no history.
- **Source**: `session_history` rows with `content_type = 'Response'` in `discord__…` or `telegram__…`, grouped by `agent_id`, `channel`, `hour`.
- **Text**: `Replied once in a Discord conversation.` / `Replied {n} times in a Discord conversation.`; same for Telegram.
- **Tone**: `default`.
- **Href**: `/{agent}/chat?channel={channel}`.
- **Key**: `reply:{agent_id}:{channel}:{hour}`; concurrent projections are idempotent.
- **Current hour**: not projected until it closes. A batch that closes after activation appears on the next list/count poll (within 15 s while the app is open, plus the hour boundary).
- **30-day cap**: a long downtime catches up only the retained window.

## WebUI chat replies

- **When**: the `Chat`/`AudioChat`/`ClientEvent` arm of `agents/process.rs` gets a saved `Message` or `AudioReply` back from `VizierAgent::chat`, and the session's channel is `VizierChannelId::HTTP` (the WebUI uses `vizier-webui`).
- **Audience**: the person whose conversation it is. The session's channel carries their username, which the service resolves through `UserStorage::get_user` to the id read state uses. The row is invisible to every other viewer of the agent (list, counts, read, mark-all).
- **Text**: `Replied in your conversation.` Never the topic title or the model's text.
- **Tone**: `default`. `Abort`/`Empty` and failed turns produce no row; the conversation itself shows the failure.
- **Href**: `/{agent}/chat/{topic}` (or the agent's chat when the session has no topic).
- **Not**: Discord/Telegram turns (their hourly `channel_replies` aggregate covers them), or a woken `BackgroundReport` turn (its `background_job` row already exists). A resumed-after-restart turn notifies like any other.
- **Notification is immediate**: it does not wait for the person to leave the conversation; read state marks nothing on open.

## Agent setup ready

- **When**: the supervisor has a built agent and either `setup_downloaded = true` or it is recovering from a failure episode (`failures > 0`), after it records `Running` (this includes a `prebuilt` update and a create build once persisted).
- **Text**: `{name} is ready.`; tone `success`; href `/{agent}/chat`.
- **Key**: `setup_ready:{agent_id}:{build_id}` (a UUID generated for the build).
- **Not when**: the build downloaded nothing and there was no preceding failure (a quick restart/config save), the build failed, the agent was deleted, or the build was stopped by shutdown/delete.
- **No duplicate**: one ready per build/episode; a retry that recovers from a failure gets its one ready even if the retry itself reused the cached artifacts.

## Agent start failure

- **When**: `Supervisor::run` fails with no failure episode open (the first failure after a successful start, or the first attempt). Create one notification and keep the episode id for retries; a successful start clears it, so a later failure is a new episode. Retries of the same episode do not add another.
- **Text**: `Failed to start: {error}. Retrying in {n}s.` (retry seconds at creation; if no next retry, omit the sentence).
- **Tone**: `failed`; href `/{agent}/config`.
- **Key**: `start_failed:{agent_id}:{episode_id}` (UUID opened with the episode).
- **Dedupe with setup**: the setup board's `failed` phase does not create a notification; the supervisor is the single failure source for persisted agents.
- **Create failure**: a build that fails before the agent is persisted is reported by the create form; no notification is left for an agent that never existed.
- **Delete/shutdown**: a stop requested by delete or shutdown does not notify; the supervisor checks its `deleting` flag and the shutdown path.

## Cross-cutting rules

- **Best effort**: every creation is `if let Err(e) = notifications.… { tracing::warn!(…) }`. A notification write never fails the source.
- **Agent check**: `Notifications::record` verifies the agent still exists and skips otherwise; `delete_agent` removes existing rows. This closes the delete/insert race.
- **No model call**: none of the wording or projection asks a model or starts a turn.
- **No content leak**: summaries use stored facts (task title/state, journal count, job kind/state/reason, integration name/action/actor, channel/count). No secrets, settings values, message bodies or model text.
- **No setup progress writes**: live setup stays in `SetupBoard` and `/utils/setup-state`; only ready/failure outcomes become rows.
- **Audience**: only `chat_reply` rows are addressed to one person; every other creation leaves `audience_user_id` empty and relies on agent visibility.
- **Ordering**: `(created_at, id)`; the event time is the transition time (the reply bucket's last reply for channel replies).
