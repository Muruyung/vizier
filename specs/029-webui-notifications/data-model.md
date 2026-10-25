# Data Model: WebUI Notifications

**Feature**: `029-webui-notifications` | **Plan**: [plan.md](./plan.md) | **Spec**: [spec.md](./spec.md)

The model has two persisted tables (the notification and a person's read mark), one existing state key (the channel-reply projection watermark), and one live, unpersisted entity (setup activity). Notifications are stored once per event; read state is per person.

## 1. Notification

A notable event, stored at the moment it completes. The row is independent of the source row afterwards: deleting a task or a dream does not delete what the inbox already recorded; deleting the *agent* does.

| Field | Type | Rules |
|---|---|---|
| `id` | `i64` | `INTEGER PRIMARY KEY AUTOINCREMENT`. Public id, stable for cursors and read marks. Never reused. |
| `event_key` | `String` | Required, non-empty, `UNIQUE`. The exactly-once guard (`task_run:812`, `job:…`, `reply:…`). Not exposed by the API. |
| `agent_id` | `String` | The agent the event concerns. Must exist when the row is inserted; deletion of the agent deletes the row. |
| `category` | `NotificationCategory` | `agent_start_failure`, `agent_setup_ready`, `task_run`, `dream_cycle`, `background_job`, `integration`, `channel_replies`. Stored as its snake-case string. |
| `tone` | `NotificationTone` | `default`, `failed` or `success`. `failed` is used for non-answered task runs, undelivered/lost jobs and start failures; `success` only for a downloaded setup that became ready. |
| `summary` | `String` | The one-sentence row text, built from stored facts at creation. Never model output; never a secret, provider key or integration setting value. |
| `href` | `String` | The in-app destination, e.g. `/{agent}/tasks?task=…&run=…`, `/{agent}/config`, `/{agent}/chat`. |
| `created_at` | `DateTime<Utc>` | Event time, stored as milliseconds. Ordering and retention use it. |

Rules:

- `event_key` is unique across the table; a retried transition is a no-op.
- A notification is created only after its source transition succeeded. A failed source write creates nothing.
- A notification write is best-effort from the source's perspective: failure is logged, not propagated.
- The summary is plain text. The WebUI renders it as text, never HTML/Markdown.
- The row is inserted only if the agent still exists. This closes the delete/insert race; `delete_agent` removes existing rows in the same transaction.

### `NotificationCategory` → wording, tone and destination

| Category | When created | Text (agent is the row title) | Tone | `href` |
|---|---|---|---|---|
| `agent_start_failure` | Supervisor enters a new failure episode | `Failed to start: {error}. Retrying in {n}s.` | `failed` | `/{agent}/config` |
| `agent_setup_ready` | Supervisor starts a build that downloaded at least one artifact, or recovers from a failure episode | `{name} is ready.` | `success` | `/{agent}/chat` |
| `task_run` | A run closes | `Task “{title}” answered.` / `… ended without an answer.` / `… was interrupted.` | `default` / `failed` / `failed` | `/{agent}/tasks?task={slug}&run={id}` |
| `dream_cycle` | Consolidation ends and the cycle has journal entries | `Dreamed: 1 journal entry.` / `Dreamed: {n} journal entries.` | `default` | `/{agent}/dream?cycle={cycle_id}` |
| `background_job` | A job reaches a terminal state | `Background batch reported back.` / `Delegation was cancelled ({reason}).` / `… finished, but its report could not be delivered.` / `… was lost when vizier restarted.` | `default` for reported/cancelled; `failed` for undelivered/lost | `/{agent}/chat` |
| `integration` | An integration or grant write commits | `{who} added/changed/switched on/switched off/deleted/unlinked {name}.` / `{who} let the agent manage its own {name} integrations.` / `… stopped …` | `default` | `/{agent}/integrations/{channels|addons|mcp}` |
| `channel_replies` | A closed hourly conversation batch is projected | `Replied once in a Discord conversation.` / `Replied {n} times in a Telegram conversation.` | `default` | `/{agent}/chat?channel={channel}` |

Pluralization, quotes, the actor-name fallback and the family→page mapping live in `src/notifications/text.rs`; one test per branch.

## 2. Read state

One row per `(person, notification)` that has been read.

| Field | Type | Rules |
|---|---|---|
| `user_id` | `String` | The person. Not foreign-keyed (users are managed by `UserStorage`); stale rows are harmless and cascade with the notification. |
| `notification_id` | `i64` | References `notification(id) ON DELETE CASCADE`. |
| `read_at` | `DateTime<Utc>` | When the person marked it read, stored as milliseconds. |

Rules:

- **Unread is the absence of a row.** Creating a notification writes nothing here; it is unread for every person who may see it.
- Mark read is `INSERT … ON CONFLICT DO UPDATE SET read_at`, or `INSERT OR IGNORE` plus a timestamp update; idempotent.
- Mark unread is `DELETE` of the pair.
- Mark all is one `INSERT OR IGNORE … SELECT` of every visible, retained notification for that user. New rows created after the statement remain unread.
- A person who gains access to an agent later sees its retained notifications as unread (no read row exists).
- A person's read choice never writes to another person's rows.

## 3. Domain and API types

### Enums

| Type | Values | Notes |
|---|---|---|
| `NotificationFilter` | `all`, `unread`, `read` | Query value; default `all`. |
| `NotificationTone` | `default`, `failed`, `success` | Serialized snake_case. |
| `NotificationCategory` | seven values above | Serialized snake_case; exposed for semantics/tests, not offered as a filter. |

### `NotificationItem` (API)

| Field | Type | Notes |
|---|---|---|
| `id` | `i64` | Public id. |
| `at` | RFC 3339 string | `created_at`. |
| `agent_id` | `String` | The client resolves the agent name from its agent store; it falls back to the id if the agent is not loaded. |
| `category` | string | One of the seven. |
| `tone` | string | `default`, `failed` or `success`. |
| `text` | `String` | The stored `summary`. |
| `href` | `String` | The destination. |
| `read` | `bool` | `LEFT JOIN notification_read` for this user. |

### `NotificationCounts` and `NotificationPage`

| Type | Fields |
|---|---|
| `NotificationCounts` | `all: u64`, `unread: u64`, `read: u64` — over the visible agents and the 30-day window, independent of the selected filter. |
| `NotificationPage` | `items: [NotificationItem]`, `counts: NotificationCounts`, `next: Option<String>` (a cursor; absent on the last page). |

### Cursor

- The cursor is URL-safe base64 (no padding) of `{"at": <millis>, "id": <i64>}`.
- Ordering is `(created_at DESC, id DESC)`; paging is `WHERE (created_at, id) < (cursor.at, cursor.id)`.
- A malformed cursor is a 400. Same-time notifications page deterministically.
- The API over-fetches one row to know whether `next` exists.

### `NotificationDraft` (internal, not exposed)

`event_key`, `agent_id`, `category`, `tone`, `summary`, `href`, `created_at`. The `Notifications` service builds it and calls the storage batch insert.

## 4. Channel-reply projection

Channel replies are the one aggregate source. The inbox stores one notification per `(agent_id, channel, hour)`.

| Field | Type | Notes |
|---|---|---|
| `agent_id` | `String` | Conversation owner. |
| `channel` | `String` | `discord__…` or `telegram__…`. |
| `hour` | `i64` | Hours since the Unix epoch, from `timestamp / 3_600_000`. |
| `replies` | `u64` | `COUNT(*)` of `Response` rows in that bucket. |
| `at` | `DateTime<Utc>` | `MAX(timestamp)` in the bucket; becomes `created_at`. |

Projection rules:

- The source is `session_history` with `content_type = 'Response'` and a Discord/Telegram channel, grouped by agent, channel and hour.
- Only **closed** hours are projected (`hour <= current_hour - 1`); the current partial hour waits.
- A global state key `notifications.replies_through` holds the last projected hour.
  - First activation: set it to `now_hour - 1` and return. No historical replies become unread.
  - Later: project `from = max(through + 1, now_hour - 30*24)` through `now_hour - 1`, insert with `INSERT OR IGNORE`, then set `through = now_hour - 1`.
- Concurrent projections may repeat work but cannot duplicate rows (`event_key UNIQUE`).
- The projection is global across agents; visibility filters at read. It runs before list, count and read-all so the badge includes a batch that just closed.

## 5. Setup activity (not persisted)

Live setup is a projection of `GET /api/v1/utils/setup-state`, not a notification.

### `SetupState` changes

| Field | Type | Change |
|---|---|---|
| `agent_id` | `String` | Existing. |
| `name` | `String` | **New**: the agent's display name, from the build's config. Used by the panel because a create build may not be in the agent store yet. |
| `phase` | `starting` / `downloading` / `ready` / `failed` | Existing, unpersisted; terminal entries linger 5 s. |
| `message` | `String` | Existing current step. |
| `error` | `Option<String>` | Existing. |
| `downloads` | `DownloadStatus[]` | Existing; kind, label, completed, total, status, error. |
| `owner_id` | `Option<String>` | Existing internal field; not serialized; decides visibility of a not-yet-persisted create build. |

### Setup board additions

- `Entry.name` and `Entry.download_epoch: u64` (monotonic; incremented by `download_started`).
- `SetupBoard::download_epoch(agent_id)` for the build to compare before/after.
- `BuildGuard::finished`/`BuiltAgent` carry `setup_downloaded` and a `build_id`; the supervisor uses them for the ready notification.
- Overlapping builds keep the download epoch of the running build; a new top-level build resets it.

### Outcome rules

- `agent_setup_ready` when `setup_downloaded` is true **or** the supervisor is recovering from a failure episode (`failures > 0`), and the agent exists.
- `agent_start_failure` once per supervisor failure episode; the episode opens at the first failure after a successful start, retries reuse it, and a successful start clears it so a later failure is a new episode. The setup board's own `failed` phase does not create a notification.
- A build stopped by shutdown or delete does not notify.
- A create build that is never persisted leaves no notification (live activity may have been visible to its owner).
- A quick restart with no download and no preceding failure leaves only live activity; no persistent row.

## 6. Lifecycle

```
                 create (source transitions)
                            │
                            ▼
                       unread (no row)
                 read ───────────────► notification_read row
                   ▲                         │
                   └──── mark unread ────────┘
                            │
              retention (30 d) or agent delete
                            ▼
                         deleted
```

| Event | Effect |
|---|---|
| Source completes | Insert unread notification (unless `event_key` exists or the agent is gone). |
| Person opens a notification's destination | Insert read row for that person. |
| Person marks read/unread | Insert/delete that person's read row. |
| Person marks all read | Insert read rows for every visible retained unread notification. |
| Access revoked | Notifications remain but are excluded by visibility on the next read. |
| Agent deleted | Delete the agent's notification rows; their read rows cascade. |
| 30 days pass | Excluded from list/count; purged at startup and daily. |
| First activation | Badge starts at zero; the reply watermark starts at the last closed hour; no backfill. |

## 7. Visibility and access

- `visible_agents(user)` is the existing rule used by Home: ownership, sharing and permissions. It is resolved in the HTTP handler before every list/count/read/read-all.
- Storage methods take `agents: &[String]` and return only rows whose `agent_id` is in it. An empty whitelist returns empty items and zero counts.
- Setting read on a notification outside the whitelist returns “not found” (404); the API never reveals that the row exists.
- The setup endpoint already applies the same visibility rule (including `owner_id` for a create build). No notification uses `owner_id`; it is live only.
- Agent deletion is checked in the service before insert, so a race cannot leave a new row after the delete transaction.

## 8. Storage queries

### List

```
SELECT n.id, n.agent_id, n.category, n.tone, n.summary, n.href, n.created_at,
       (r.notification_id IS NOT NULL) AS read
FROM notification n
LEFT JOIN notification_read r
  ON r.notification_id = n.id AND r.user_id = ?user
WHERE n.agent_id IN (…visible…)
  AND n.created_at >= ?cutoff
  AND (?filter = 'all' OR (?filter = 'unread' AND r.notification_id IS NULL)
                        OR (?filter = 'read'   AND r.notification_id IS NOT NULL))
  AND (?cursor IS NULL OR (n.created_at, n.id) < (?cursor_at, ?cursor_id))
ORDER BY n.created_at DESC, n.id DESC
LIMIT ?limit;
```

### Counts

```
SELECT COUNT(*) AS all,
       SUM(CASE WHEN r.notification_id IS NULL THEN 1 ELSE 0 END) AS unread,
       SUM(CASE WHEN r.notification_id IS NOT NULL THEN 1 ELSE 0 END) AS read
FROM notification n
LEFT JOIN notification_read r
  ON r.notification_id = n.id AND r.user_id = ?user
WHERE n.agent_id IN (…visible…) AND n.created_at >= ?cutoff;
```

### Mark all

```
INSERT OR IGNORE INTO notification_read (user_id, notification_id, read_at)
SELECT ?user, n.id, ?now
FROM notification n
WHERE n.agent_id IN (…visible…) AND n.created_at >= ?cutoff;
```

### Mark one

- Verify the notification's `agent_id` is visible.
- Read: `INSERT INTO notification_read … ON CONFLICT(user_id, notification_id) DO UPDATE SET read_at = excluded.read_at`.
- Unread: `DELETE FROM notification_read WHERE user_id = ? AND notification_id = ?`.
- Return whether a visible row was found.

### Purge

```
DELETE FROM notification WHERE created_at < ?cutoff;  -- read rows cascade
```

## 9. Validation and invariants

- `event_key`, `summary`, `href` are non-empty; category/tone parse to a known value.
- `created_at` is stored in milliseconds; the API serializes RFC 3339.
- `limit` is clamped to 1–50 (default 20); an invalid filter or cursor is a 400.
- Mark read on a missing/invisible id is a 404; storage returns `false`.
- A page never repeats or skips a row across `next` cursors, including ties.
- `notification_read` never contains a row for a deleted notification (cascade).
- No row contains a secret value, provider key, integration setting value or model-written text.
- At most one row per `event_key`; one row per `(agent, channel, hour)` for replies.
- The unread badge equals the `unread` count for the same person/visibility (capped only for display).
- Notifications older than 30 days are neither listed nor counted.

## 10. Storage trait

```rust
#[async_trait]
pub trait NotificationStorage {
    /// Idempotent batch insert on event_key; returns how many were new.
    async fn insert_notifications(&self, drafts: &[NotificationDraft]) -> Result<usize>;

    /// Newest first, filtered by state and the visible agent whitelist.
    async fn list_notifications(
        &self,
        user_id: &str,
        agents: &[String],
        filter: NotificationFilter,
        before: Option<(DateTime<Utc>, i64)>,
        limit: usize,
    ) -> Result<Vec<NotificationItem>>;

    async fn notification_counts(
        &self,
        user_id: &str,
        agents: &[String],
    ) -> Result<NotificationCounts>;

    /// Read/unread for one visible notification; false when missing or not visible.
    async fn set_notification_read(
        &self,
        user_id: &str,
        notification_id: i64,
        read: bool,
        at: DateTime<Utc>,
        agents: &[String],
    ) -> Result<bool>;

    /// Marks every visible, retained unread notification read; returns rows inserted.
    async fn mark_all_notifications_read(
        &self,
        user_id: &str,
        at: DateTime<Utc>,
        agents: &[String],
    ) -> Result<u64>;

    /// Closed-hour reply aggregates for all agents, used by the projection.
    async fn channel_reply_batches(
        &self,
        from_hour: i64,
        to_hour: i64,
    ) -> Result<Vec<ReplyBatch>>;

    async fn purge_notifications_before(&self, at: DateTime<Utc>) -> Result<usize>;
}
```

`VizierStorage` forwards every method to the concrete backend (SQLite only today), and `VizierStorageProvider` lists `NotificationStorage` alongside the other concerns. `ReplyBatch` is an internal storage type (`agent_id`, `channel`, `hour`, `replies`, `at`).
