# Contract: Notification HTTP API

**Feature**: `029-webui-notifications` | **Plan**: [../plan.md](../plan.md) | **Data model**: [../data-model.md](../data-model.md)

All routes are under `/api/v1`, require a signed-in person, and are global across the agents that person may see. The response envelope is the existing `APIResponse<T>` (`status`, `message`, `data`). There is no notification permission beyond authentication and agent visibility.

## Visibility and access

- Every handler resolves `visible_agents(user)` before touching notifications (the same rule Home uses: ownership, sharing, permissions).
- A notification is returned only when its `agent_id` is in that set. A `chat_reply` is additionally returned only to the person whose conversation it is (its `audience_user_id`); everyone else sees it nowhere, including counts and per-id reads.
- There is no per-user notification row; read state is per person.
- A read/read-all request never touches another person's read state or another agent's rows.
- A missing, invisible or other-audience notification id on a per-id route is `404` (it does not reveal that the id exists).
- Notifications older than 30 days are never returned or counted.

## `GET /api/v1/notifications`

The inbox page, newest first.

### Query

| Name | Type | Rules |
|---|---|---|
| `filter` | `all` \| `unread` \| `read` | Optional, default `all`. Anything else is `400`. |
| `before` | string | Optional opaque cursor: the `next` of the previous page. Malformed → `400`. |
| `limit` | integer | Optional, clamped to 1–50; default 20. |

### Response `NotificationPage`

```json
{
  "status": 200,
  "data": {
    "items": [
      {
        "id": 42,
        "at": "2026-10-25T09:14:02Z",
        "agent_id": "scribe",
        "category": "task_run",
        "tone": "failed",
        "text": "Task “Morning digest” ended without an answer.",
        "href": "/scribe/tasks?task=morning-digest&run=812",
        "read": false
      },
      {
        "id": 41,
        "at": "2026-10-25T08:55:10Z",
        "agent_id": "scribe",
        "category": "agent_setup_ready",
        "tone": "success",
        "text": "Scribe is ready.",
        "href": "/scribe/chat",
        "read": true
      }
    ],
    "counts": { "all": 26, "unread": 3, "read": 23 },
    "next": "eyJhdCI6MTc2MTM4MDQ0MjAwMCwiaWQiOjQyfQ"
  }
}
```

- `counts` are for the whole visible, retained set, **independent of `filter`**.
- `next` is absent/null on the last page.
- The order is `(created_at, id)` descending; equal timestamps never reorder between pages.
- `read` is this person's state.
- `text` is plain text; clients render it as text.

## `GET /api/v1/notifications/count`

A cheap poll target for the badge.

```json
{ "status": 200, "data": { "all": 26, "unread": 3, "read": 23 } }
```

The same counts rules as the list. A failure leaves the client's last known count intact; it must not be treated as zero.

## `PUT /api/v1/notifications/{id}/read`

Marks one visible notification read or unread.

```json
{ "read": true }
```

```json
{ "status": 200, "data": { "all": 26, "unread": 2, "read": 24 } }
```

- `read` is required and must be a boolean; a missing/!boolean body is `400`.
- `true` inserts/updates this person's read row; `false` deletes it.
- Idempotent: repeating either request returns the same counts.
- `404` when the id is missing, deleted, outside retention, its agent is not visible, or it is another person's `chat_reply`.
- The response counts let the client update the badge and filters without a second request.

## `POST /api/v1/notifications/read-all`

Marks every visible, retained unread notification read for the caller, including ones not loaded in the page. No body.

```json
{ "status": 200, "data": { "all": 26, "unread": 0, "read": 26 } }
```

- The projection runs first, so a reply batch that has just closed is included.
- Idempotent; no unread rows is a success with unchanged counts.
- A notification created after the statement's snapshot remains unread.

## Live setup data

The panel's live setup section uses the existing:

`GET /api/v1/utils/setup-state` → `{ setups: SetupState[] }`

`SetupState` gains one field:

```json
{
  "agent_id": "scribe",
  "name": "Scribe",
  "phase": "downloading",
  "message": "Pulling qwen3:32b",
  "error": null,
  "downloads": [
    {
      "kind": "model",
      "label": "qwen3:32b",
      "completed": 8200000000,
      "total": 20000000000,
      "status": "pulling manifest",
      "error": null
    }
  ]
}
```

- The endpoint already filters by visibility, including a not-yet-persisted create build's owner.
- It is not a notification endpoint: live setup has no read state and no counts.
- The client polls it (1 s while visible) and computes percent/ETA with the existing helpers.

## Errors

| Status | When |
|---|---|
| `400` | Unknown `filter`, malformed `before`, missing/non-boolean `read`. |
| `401` | No valid session. |
| `404` | Per-id read of a missing, invisible, deleted or expired notification. |
| `500` | Storage failure; the client keeps its last known counts and offers retry. |

## Removed

`GET /api/v1/overview/feed` is removed with this feature. `/api/v1/notifications` and the Home page's removal of the away panel replace it. This is a deliberate breaking HTTP API change, called out in the commit/changelog.

## OpenAPI

Add `NotificationItem`, `NotificationPage`, `NotificationCounts`, `NotificationReadRequest` and the `NotificationFilter` query to `ApiDoc`, and add `SetupState.name`. Regenerate `webui/openapi.json` and `webui/app/api/schema.d.ts` after the Rust change.
