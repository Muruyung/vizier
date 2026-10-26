# Research: Standalone Dummyplug

## R1 — Compatibility boundary

**Decision**: Treat [legacy §1–§6](../008-dummyplug-provider/contracts/dummyplug-protocol.md) and the [context supplement](../013-reaction-awareness/contracts/dummyplug-context.md) as golden behavior. Port all current capabilities first, including text normalization, tool-list/sample formatting, JSON validation, tool-result echo, context and random lorem. Preserve the default tool call even for `tool_choice: none`; honoring it requires an explicit test control.

**Rationale**: The constitution makes existing dummyplug quickstarts a release gate. The current `DummyplugModel` parses prepared message parts, strips frontmatter and attached-file trailers, and recognizes fences and a separate `# Context` part. Server inputs travel through OpenAI Chat encoding, which can alter that representation; parity needs tests at both boundaries. In particular `joined_text` in `src/llm/dialect/openai_chat.rs` emits an omitted-attachment marker for tool-result attachments, whereas the current model renders attachment parts as `[image]`. Capture the old output before removal and address this difference without a new per-provider dispatch branch.

**Alternatives considered**: Approximate user-visible text from the issue; rejected because it misses edge cases and creates test regressions. Keep the in-process model as fallback; rejected because it bypasses the transport path and hides missing-server failures.

## R2 — Provider integration and URL resolution

**Decision**: Use the existing `OpenAiChat` dialect with base URL `http://127.0.0.1:9998/v1`. Resolve `DUMMYPLUG_URL` into a BaseUrl credential through the generic builtin field/env mechanism. Expose normal provider GET/PUT configuration so WebUI can edit `base_url`. Keep the static `dummyplug` model fallback and accept arbitrary agent model strings.

**Rationale**: `src/agents/agent/model/mod.rs` currently constructs `DummyplugModel` directly; `src/llm/resolve.rs` rejects dummyplug; `src/catalogue/builtin.rs` has no dialect or configurable fields; provider HTTP API explicitly hides it. All four exclusions must change together. `src/provider_keys.rs` currently reads builtin env names only for secret-like fields, so adding `DUMMYPLUG_URL` to catalogue metadata alone will not work. The generic credential resolution needs a field-aware fix and tests. `OpenAiChat` already sends chat completion requests, parses whole JSON replies, and probes `/models`.

**Alternatives considered**: New dummyplug-only HTTP client; rejected as duplicate provider plumbing. Hardcode env override in resolver; rejected under trait-based extensibility. Make Vizier consume SSE immediately; not required because its current OpenAI Chat dialect expects whole JSON, while external SDKs can use server SSE.

## R3 — Server protocol and deterministic response generation

**Decision**: Generate one canonical completion from normalized request + explicit seed + scenario state, then serialize it as whole JSON or SSE deltas. Include standard id/model/finish_reason/usage fields; emit `data: [DONE]` and an optional usage chunk when `stream_options.include_usage` is requested. Send tool-call arguments as JSON-encoded strings in function-call deltas. Use the same canonical answer for both modes.

**Rationale**: This makes the spec's whole/stream equivalence testable and avoids two response engines. The current Vizier OpenAI Chat decoder expects a first-choice message with content or tool calls. The [official OpenAI Chat Completions reference](https://platform.openai.com/docs/api-reference/chat) describes the wire envelope, SSE stream and optional usage chunk.

**Alternatives considered**: Generate SSE token-by-token with separate random calls; rejected because the text can differ from whole replies under the same seed. Introduce a new protocol version; rejected because contract tests cover compatibility and there is one target client dialect.

## R4 — Stateful test controls

**Decision**: Put state behind a test-session ID. Commands use a `DUMMYPLUG` prefix; per-request headers cover seed, timing, usage and scenario selection; `/_dummyplug/*` supplies reset, scenario programming and request inspection. The same session's operations are ordered, separate sessions do not consume each other's scripts, and exhausted scripts fall back to legacy behavior. Bound entries, bytes and inactive lifetime; reset removes logs and scenario state. Limit control/inspection endpoints to local callers unless a separate control token is configured. Ignore public chat Authorization.

**Rationale**: Tests can run concurrently and inspect actual transport payloads without writing prompts to Vizier storage. Remote exposure should not make private prompt logs readable merely because chat is keyless. Failure attempts need a stable scenario/session key so repeated requests advance to success. A dropped request must cancel sleep, hold and remaining stream output.

**Alternatives considered**: Process-global sequence and unbounded log; rejected because tests interfere and private data accumulates. Durable SQLite/Redis state; rejected as unnecessary and contrary to the optional service's simplicity. Reuse chat API key for inspection; rejected because chat intentionally needs no key.

## R5 — Delivery and gates

**Decision**: Keep the server in `vizier-lab/dummyplug`, pin it as a submodule, and merge its PR before the dependent Vizier PR. The e2e harness explicitly starts the server; `just run` starts only Vizier. Compose uses an internal service URL through `DUMMYPLUG_URL`. CI tests the pinned server, a stock SDK, and Vizier's dummyplug agent path. Update submodule cascade automation to know `dummyplug → vizier` without disturbing the existing design/webui chain.

**Rationale**: An independent release artifact proves the server runs without Vizier. The gitlink makes CI reproducible. Existing `scripts/client-tools-e2e.mjs` and `scripts/durable-turns-e2e.mjs` assume a running Vizier/JWT/dummyplug agent; the new harness must provision those dependencies and handle Vizier restart tests. Repository search has not confirmed access to `vizier-lab/dummyplug`; resolve creation/permissions before implementation.

**Alternatives considered**: A Vizier workspace member or supervised child process; rejected because the normal build/startup would depend on dummyplug. Floating submodule branch; rejected because CI and review would test different server revisions.
