# Quickstart and Acceptance: Standalone Dummyplug

These commands and checks are the acceptance script for the implementation PR. The `dummyplug/` submodule and `just dummyplug` recipe are planned interfaces; they are not part of this spec-only branch.

Use the [completion gate](spec.md#completion-gate) for required evidence. Run the implementation checks against the exact dummyplug commit pinned by the Vizier gitlink; publish that commit before testing a fresh recursive checkout or CI.

## 1. Start both processes locally

From a recursive Vizier checkout, run in separate terminals:

```sh
just install
just dummyplug
```

```sh
env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY just run
```

Expect `curl http://127.0.0.1:9998/healthz` to be healthy and `curl http://127.0.0.1:9998/v1/models` to include `dummyplug`. `just run` starts Vizier only. Stop dummyplug, restart Vizier, and verify Vizier still starts; a dummyplug turn then reports a provider connection failure. Restart dummyplug for the remaining checks.

## 2. Check the public API without Vizier

```sh
curl -sS http://127.0.0.1:9998/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -H 'X-Dummyplug-Seed: 42' \
  -d '{"model":"dummyplug","messages":[{"role":"user","content":"hello"}]}'
```

Expect a Chat Completion with assistant text, model, finish reason and zero usage. Repeat the same request and seed: observable output must match. Repeat with `"stream":true` and `"stream_options":{"include_usage":true}`; the content reconstructed from SSE deltas equals the whole answer, the stream ends with `[DONE]`, and the optional usage chunk has empty `choices`. Also run the same model-list/chat/stream/tool-call/tool-result sequence with a stock OpenAI-compatible SDK pointed at this base URL and a dummy API key if its constructor requires one.

## 3. Run the constitutional dummyplug-agent gate

In WebUI, create or reuse an agent with provider `dummyplug` and model `dummyplug` (an existing agent needs no edits). Send the following in one topic:

| Send | Expected reply or effect |
|---|---|
| `hello` | 1–3 nonempty lorem paragraphs. |
| `tools` | Sorted, formatted available-tool list. |
| An exact tool name from that list, such as `READ_CORE` | Full description and fenced sample JSON. |
| Paste the generated JSON request | One actionable tool call, tool history entry, then `**Tool result** (...)` echo. |
| `{"tool":"nope"}` | Unknown-tool assistant text; no tool runs. |
| `{"tool":` | Parse-error assistant text; no tool runs. |
| `context` | The verbatim current `# Context` block or `(no context block)`. |

Repeat the tool-name/JSON step with a state-changing tool such as `memory_write`; verify its stored result through the existing UI. Run the established dummyplug quickstarts and `scripts/client-tools-e2e.mjs` and `scripts/durable-turns-e2e.mjs` through the new harness. Compare all legacy §1–§6 goldens, including frontmatter, attached-file markers, fenced JSON, multiple trailing tool results, tool schema ordering, and `tool_choice: none`. No live model credential is required.

## 4. Exercise controls and isolation

Use two distinct `X-Dummyplug-Session` IDs. Program a two-failure-then-success step for session A through `PUT /_dummyplug/scenarios/A`; send three equal requests and expect the selected error status twice and a normal completion third, with configured `Retry-After` on failures. Session B remains on its independent script. Read `GET /_dummyplug/requests?session_id=A` and confirm messages, tools, tool choice and token limit are visible but Authorization is not. `POST /_dummyplug/reset` clears both log and state for the target session.

Send a held request, cancel the client, and verify no later response or lingering task. Test an injected truncated SSE stream: it must lack a normal finish and `[DONE]`. Set nonzero usage and ensure the response reports it. Confirm the unmodified request still returns legacy zero usage, and that a plain `HOLD` yields lorem while an explicit `DUMMYPLUG` command activates a control.

## 5. Run project and deployment gates

```sh
cd dummyplug && cargo test && cargo clippy
```

From Vizier's root:

```sh
cargo test
cargo clippy
node --test scripts/*.test.mjs
```

Run the CI e2e harness against the pinned submodule commit and run Vizier with `DUMMYPLUG_URL=http://127.0.0.1:9998/v1`. Use `docker-compose.yaml` with the dummyplug service and verify Vizier reaches `http://dummyplug:9998/v1` inside Compose. Build and start the normal Vizier release without building or starting dummyplug; non-dummyplug agents must still work.

## 6. Review order

Review and merge the standalone `vizier-lab/dummyplug` PR first. Then update the `dummyplug/` gitlink in the Vizier PR, rerun all gates against that exact pinned commit, and review the provider, harness, documentation and cascade changes together.
