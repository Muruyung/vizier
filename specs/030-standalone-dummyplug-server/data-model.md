# Data Model: Standalone Dummyplug

All server state is in memory and expires; no entity below changes Vizier's durable agent schema.

| Entity | Fields | Rules |
|---|---|---|
| Chat request | `model`, ordered `messages`, `tools`, `tool_choice`, token limit, `stream`, `stream_options`, optional controls | Accept text string or text parts; preserve message/tool order. Unknown model string is accepted. Malformed wire payload returns a client error; malformed legacy JSON command returns assistant text per §3. |
| Normalized message | role, text parts, optional tool-call ID/name/arguments, optional tool result, context part | Normalize frontmatter/trailer/fences for legacy command matching; keep raw request separately for inspection. Resolve a tool result name from prior assistant call, else use call ID. |
| Canonical completion | stable ID, requested model, assistant content or tool calls, finish reason, usage | One generation pass feeds whole and SSE serializers. Default legacy usage is zero. Seeded repetitions produce identical observable result. |
| Scenario | session ID, queued steps, next index, last activity, failure attempt counts | Session ID isolates parallel tests. Script steps are consumed atomically in arrival order. Exhaustion returns legacy default. Explicit reset and inactivity TTL remove state. |
| Scenario step | optional response text/tool calls/reasoning/finish reason, delay/hold, failure status/count/retry-after, usage, stream termination | A failure count decrements only for its own session/step and succeeds after N failures. A hold completes only on release/cancel; disconnect cancels work. |
| Request record | timestamp, session ID, request messages/tools/tool choice/token limit, relevant non-secret control headers, outcome | Bounded ring by count and bytes; redact Authorization and control token. Local or token-authorized inspection only. Reset clears records. |
| Provider configuration | provider ID `dummyplug`, `base_url`, optional env override | Default `http://127.0.0.1:9998/v1`; user-configured URL and `DUMMYPLUG_URL` follow the ordinary provider precedence. Existing agent provider/model fields need no migration. |

## State transitions

```text
Scenario: absent → configured → active → exhausted → expired/reset
Failure step: remaining=N → ... → remaining=1 → success → next step
Held request: waiting → released → response; waiting → client disconnect → cancelled
Stream: open → chunks → final finish/optional usage → [DONE]; open → disconnect/injected truncation → aborted
```

## Invariants

- No test control changes a request without an explicit command, header, or programmed scenario.
- Legacy command precedence is `tools`, `context`, exact tool name, JSON tool request, then `DUMMYPLUG` command and fallback text; a new command cannot shadow an existing one.
- A scenario step is selected within its session only. Request inspection is observational and cannot alter the reply.
- A failed HTTP attempt and a cancelled stream never appear as a successful completion.
- Limits and TTL have safe finite defaults; implementation tests exercise eviction and expiration.
