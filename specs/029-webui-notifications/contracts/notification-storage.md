# Contract: Notification Storage

**Feature**: `029-webui-notifications` | **Plan**: [../plan.md](../plan.md) | **Data model**: [../data-model.md](../data-model.md)

`NotificationStorage` is one storage concern, implemented by `SqliteStorage`, forwarded by `VizierStorage`, and listed in `VizierStorageProvider` like every other concern. It owns the two new tables and every query behind the HTTP contract.

## DDL

Added by `init_notification_schema` from `SqliteStorage::init_schema`:

```sql
CREATE TABLE IF NOT EXISTS notification (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    event_key   TEXT NOT NULL UNIQUE,
    agent_id    TEXT NOT NULL,
    category    TEXT NOT NULL,
    tone        TEXT NOT NULL,
    summary     TEXT NOT NULL,
    href        TEXT NOT NULL,
    created_at  INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_notification_created
    ON notification(created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_notification_agent_created
    ON notification(agent_id, created_at DESC, id DESC);

CREATE TABLE IF NOT EXISTS notification_read (
    user_id         TEXT NOT NULL,
    notification_id INTEGER NOT NULL REFERENCES notification(id) ON DELETE CASCADE,
    read_at         INTEGER NOT NULL,
    PRIMARY KEY (user_id, notification_id)
);
```

Notes:

- `created_at` and `read_at` are Unix milliseconds (the convention of `task_run`, `background_job`, `session_history` and the feed).
- The connection already enables `PRAGMA foreign_keys=ON`; the read rows cascade.
- `event_key` is the idempotency key and is not exposed by the API.
- No existing table is altered. There is no data migration.
- `delete_agent` adds `DELETE FROM notification WHERE agent_id = ?` in its existing transaction, after/before the other agent-owned rows; the read rows cascade.
- The startup sweep may optionally delete any notification whose `agent_id` no longer exists in `agent_config`; the service also refuses to insert for a missing agent.

## Trait

```rust
#[async_trait]
pub trait NotificationStorage {
    async fn insert_notifications(&self, drafts: &[NotificationDraft]) -> Result<usize>;
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
    async fn set_notification_read(
        &self,
        user_id: &str,
        notification_id: i64,
        read: bool,
        at: DateTime<Utc>,
        agents: &[String],
    ) -> Result<bool>;
    async fn mark_all_notifications_read(
        &self,
        user_id: &str,
        at: DateTime<Utc>,
        agents: &[String],
    ) -> Result<u64>;
    async fn channel_reply_batches(
        &self,
        from_hour: i64,
        to_hour: i64,
    ) -> Result<Vec<ReplyBatch>>;
    async fn purge_notifications_before(&self, at: DateTime<Utc>) -> Result<usize>;
}
```

`ReplyBatch` is `{ agent_id: String, channel: String, hour: i64, replies: u64, at: DateTime<Utc> }`.

## Method semantics

### `insert_notifications`

- One transaction for the batch.
- `INSERT OR IGNORE` on `event_key`; returns the number actually inserted.
- A draft whose `agent_id` no longer exists in `agent_config` is skipped (the service also checks; this is the storage-side safety net).
- Validates `event_key`/`summary`/`href` non-empty and category/tone parseable; an invalid draft is an error for the whole batch (programmer error, not user input).
- Never updates an existing row: the same event key is the same event forever.

### `list_notifications`

- `agents` is the visibility whitelist. Empty → `[]`.
- `filter` is applied with `LEFT JOIN notification_read r ON r.notification_id = n.id AND r.user_id = ?`.
- Always `n.created_at >= now - 30 days`.
- `before` applies `(n.created_at, n.id) < (cursor_at, cursor_id)`; ordering is `n.created_at DESC, n.id DESC`.
- `limit` is the page size; the API asks for `limit + 1` to compute `next`.
- Returns `read = r.notification_id IS NOT NULL` per row.

### `notification_counts`

One query returns `all`, `unread` and `read` over the visible agents and the retention window, regardless of the selected filter. Used by the list (after the list call), the count endpoint and every read mutation response.

### `set_notification_read`

- First proves the notification exists and its `agent_id` is in `agents`; otherwise returns `false` (API 404).
- `read = true`: `INSERT … ON CONFLICT(user_id, notification_id) DO UPDATE SET read_at = excluded.read_at`.
- `read = false`: `DELETE FROM notification_read WHERE user_id = ? AND notification_id = ?`.
- Idempotent; returns `true` when the notification was visible, whether or not the state changed.

### `mark_all_notifications_read`

- One `INSERT OR IGNORE INTO notification_read … SELECT` over visible, retained notifications.
- Returns rows inserted (not the total read count).
- New notifications created after the statement's snapshot remain unread.
- The service runs the reply projection before this call.

### `channel_reply_batches`

- Global across agents (visibility is applied later).
- Selects `agent_id, channel, timestamp / 3_600_000 AS hour, COUNT(*), MAX(timestamp)` from `session_history`.
- `content_type = 'Response'`; `channel LIKE 'discord\_\_%'` or `telegram\_\_%`.
- `hour >= from_hour AND hour <= to_hour`; hours are UTC. Closed hours only are requested by the service.
- Ordered oldest first or by `(hour, agent, channel)`; the caller inserts and is idempotent.
- Uses the existing `session_history` index on `(agent_id, channel, timestamp)` where available; a 30-day catch-up is acceptable at startup/first read.

### `purge_notifications_before`

- `DELETE FROM notification WHERE created_at < ?`; read rows cascade.
- Called at startup and once a day from the scheduler tick.
- Listing/counting also filters by the window, so a delayed purge never leaks old rows into the UI.

## Projection watermark

Not a storage method; the `Notifications` service uses the existing `StateStorage`:

- Key: `notifications.replies_through`.
- Value: the last **closed** UTC hour projected, as `hour = timestamp / 3_600_000` (an integer).
- Missing on first activation: set to `now_hour - 1`, insert nothing, return. No historical replies are imported.
- Later: `from = max(through + 1, now_hour - 30*24)`, `to = now_hour - 1`; if `from > to`, return. Fetch batches, insert, then save `through = to`.
- A crash between insert and save re-inserts the same keys next time; the unique key makes it harmless.
- Concurrent calls may repeat work but not rows; SQLite serializes the writes.

## Visibility

The storage never calls the permission system itself. The HTTP handler resolves `visible_agents` and passes the ids; the storage treats them as the complete whitelist. This keeps one visibility rule (`user_can_view_agent`) and makes revocation immediate. The service's insert-time existence check is not a permission check; it only prevents rows for a deleted agent.

## Ordering and idempotency

- `id` is `AUTOINCREMENT`, so a deleted id is never reused and a cursor cannot alias a new row.
- `(created_at, id)` is the only ordering; ties page deterministically.
- `event_key` is the exactly-once guard for retried transitions (job recovery, task recovery, startup sweeps, concurrent reply projections).
- The two writes in a read action (read row + counts response) are separate reads; the counts are read after the commit, so the response is consistent enough for the UI. The optimistic UI tolerates a concurrent change.

## Tests

- Two identical drafts insert one row; the second batch reports 0 new.
- Task/dream/job/integration/reply/ready/failure rows read back with the expected `tone`, `href` and `created_at`.
- A draft for a missing agent is skipped.
- Visibility: only whitelisted agent ids appear; an empty whitelist returns empty items/counts.
- Filter counts are independent of the selected page/filter; unread + read = all.
- Read/unread toggles are idempotent; marking a non-visible id returns false.
- Mark-all covers all visible unread beyond the page, leaves a concurrently inserted row unread, and returns counts.
- Keyset pages have no duplicates/gaps, including identical timestamps.
- Retention: rows at exactly 30 days are included; older rows are not; purge deletes and cascades read rows.
- Deleting an agent deletes its notifications and read rows; other agents' rows survive.
- Channel replies group by conversation-hour, skip Requests/HTTP channels, and produce one row per bucket.
- Projection watermark: first activation imports nothing; a closed hour is projected once; a 30-day catch-up is capped; concurrent calls do not duplicate.
