# Contract: Dummyplug Test Controls

The unmodified request always follows the legacy protocol. New manual commands begin with `DUMMYPLUG` and are considered only after existing `tools`, `context`, exact tool name, and JSON tool-command recognition. A plain `HOLD` or `FAIL` remains an ordinary lorem prompt.

## Per-request controls

| Header | Meaning |
|---|---|
| `X-Dummyplug-Session` | Stable test-session ID; isolates scenario progress and request records. |
| `X-Dummyplug-Seed` | Integer seed for repeatable default text, IDs and stream partitioning. |
| `X-Dummyplug-Delay-Ms` | Delay before the first byte. |
| `X-Dummyplug-Chunk-Delay-Ms` | Delay between SSE chunks. |
| `X-Dummyplug-Usage` | JSON object containing `prompt_tokens`, `completion_tokens`, optional cached-token count. |
| `X-Dummyplug-Respect-Tool-Choice` | `true` opts into honoring `tool_choice: none`; default remains legacy behavior. |

Invalid control values return a clear non-2xx test-control error and never silently fall back. Ordinary Chat Completions headers are unaffected. The server also supports explicit `DUMMYPLUG` commands for manual delay/hold, error, fixed reply, usage, reasoning, scripted tool calls and finish reason; the precise parser grammar will be frozen by golden tests before implementation, and examples will be added to user docs. Commands that require multi-attempt state must name a session or key.

## Control and inspection API

| Method and path | Purpose |
|---|---|
| `PUT /_dummyplug/scenarios/{session_id}` | Replace the ordered scenario steps for one session. Body is JSON with `steps`; each step may define a reply/tool call, failure count/status/retry timing, delay/hold, usage or finish reason. Inactive sessions expire under the server's finite TTL. |
| `GET /_dummyplug/requests?session_id=...` | Read bounded recent request records for one session, including messages, tools, tool choice, token limit and non-secret control headers. |
| `POST /_dummyplug/reset` | Clear one named session or all sessions and request records. |
| `POST /_dummyplug/release/{session_id}` | Release that session's held requests. |

Control/inspection endpoints accept loopback callers by default. If exposed to non-local callers, the server requires a separately configured control token; the ordinary chat `Authorization` header never grants control access. Logs never return Authorization or the control token. Responses use JSON and distinguish missing session, invalid step and unauthorized access.

## State and cancellation

Scenario steps are selected atomically within a session. A step configured to fail `N` attempts returns the specified status for those `N` requests, optionally with `Retry-After`, then succeeds. Each session has independent counters. Exhausted scripts use legacy default behavior. Logs and scenarios have finite count/byte limits and inactivity TTL; reset removes both. A disconnected client cancels its hold, delay and stream task. An explicitly truncated stream ends without a successful terminator.
