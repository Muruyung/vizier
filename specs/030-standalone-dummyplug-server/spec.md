# Feature Specification: Standalone Dummyplug Test Server

**Feature Branch**: `030-standalone-dummyplug-server`

**Created**: 2026-10-25

**Status**: Ready for implementation; acceptance pending

**Input**: [Vizier issue #95](https://github.com/vizier-lab/vizier/issues/95) and the decisions agreed in this conversation: preserve all existing dummyplug behavior first; run it as a separate process; preserve legacy `tool_choice: none` behavior by default; use a local default address with an override; keep media stubs out of scope; add isolated, bounded test controls.

## Context

Dummyplug is Vizier's offline test model. It currently runs inside Vizier, so its tests bypass the provider connection, request transport, response parsing, and failure handling used by external models. A separately runnable dummyplug lets Vizier's offline tests exercise that same path and lets another client use the test model. The replacement must preserve every existing dummyplug command and response rule before adding test controls. Existing agents retain their provider and model values without a data migration.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Keep every existing dummyplug test working (Priority: P1)

A Vizier contributor runs an existing dummyplug agent against the new server. The agent still lists its available tools, describes a named tool, calls a tool from a valid JSON request, echoes the result, reveals the per-request context, and answers ordinary prompts as before. Existing quickstarts and tests work after the contributor starts the server.

**Why this priority**: The current protocol is the project's offline test harness. Losing any current behavior would make the replacement unsafe, regardless of its new features.

**Independent Test**: Run the existing §1–§6 protocol cases against the old model and the new server with equivalent requests, then run an existing dummyplug agent through Vizier. Compare exact output wherever the old result is deterministic and check the documented form of random output.

**Acceptance Scenarios**:

1. **Given** the same tool definitions, **When** a user sends `tools` or an exact tool name, **Then** the available-tool list and sample request match the current contract, including ordering, descriptions, arguments, and formatting.
2. **Given** a valid JSON tool request, **When** dummyplug handles it, **Then** Vizier receives one actionable tool call, and a later request containing its tool result returns the current echo format.
3. **Given** malformed JSON, invalid arguments, or an unknown tool, **When** dummyplug handles the request, **Then** it returns the current assistant-text error and runs no tool.
4. **Given** a request containing channel frontmatter, an attached-files trailer, a fenced command, and a separate context block, **When** dummyplug reads it, **Then** command matching and `context` behave exactly as in the existing protocol.
5. **Given** a message that is not a recognized command, **When** dummyplug answers, **Then** it retains the existing lorem-ipsum response form; a seeded test can reproduce its output.
6. **Given** an existing agent configured with provider `dummyplug` and any model string, **When** its server is available, **Then** it works without editing or migrating that agent.
7. **Given** a request that disallows tool calls, **When** a legacy JSON tool command is sent with no extra control enabled, **Then** dummyplug retains its existing tool-call behavior.

---

### User Story 2 - Use dummyplug from another client (Priority: P2)

A developer starts dummyplug without starting Vizier, checks that it is ready, lists its model, and sends a chat request using a standard OpenAI-compatible client. The client can receive a whole response or streamed pieces representing the same answer.

**Why this priority**: Independent use and the real provider connection are the reason for extracting dummyplug.

**Independent Test**: Start only dummyplug and run a conformance script with a stock OpenAI-compatible SDK. Check model listing, ordinary text, a tool call and its result, and streaming.

**Acceptance Scenarios**:

1. **Given** dummyplug is running, **When** a client checks readiness and lists models, **Then** it receives a healthy response and the `dummyplug` model.
2. **Given** a chat request with text as a string or as text parts, **When** a client requests a whole response, **Then** it receives a valid completion with model, finish reason, message, and usage fields.
3. **Given** the same request and seed, **When** a client requests streaming, **Then** the ordered chunks contain a role, content or tool-call deltas, a final finish reason, an optional usage result when requested, and an end marker; reassembled content equals the whole response.
4. **Given** a tool-call result after a prior assistant call, **When** a client sends both messages, **Then** dummyplug resolves the tool name and returns the documented echo.
5. **Given** no authorization header or an arbitrary authorization header, **When** a client uses the chat surface, **Then** neither requires a credential.

---

### User Story 3 - Exercise provider behavior offline (Priority: P2)

A test author makes dummyplug return a repeatable reply, pause, report usage, or fail for a selected number of attempts. The author inspects what Vizier actually sent and can run several isolated test conversations in parallel.

**Why this priority**: The current in-process model cannot test retries, transport failures, streaming, nonzero usage, or the exact outgoing request.

**Independent Test**: Without Vizier, configure each behavior and verify its response and inspection record. With Vizier, prove a temporary failure is retried, usage is recorded, a held request can be aborted, and parallel scenarios stay separate.

**Acceptance Scenarios**:

1. **Given** no test control, **When** a legacy request arrives, **Then** the controls do not alter its behavior or default zero usage.
2. **Given** a fixed seed, **When** an equivalent request is repeated, **Then** its generated text, identifiers, and streamed pieces are reproducible.
3. **Given** a hold or delay control, **When** the client disconnects or aborts, **Then** the pending work stops promptly and does not send a later response.
4. **Given** a failure plan for two attempts followed by success, **When** the client retries, **Then** the first two attempts receive the specified failure and the third receives a normal completion; retry timing can be checked when supplied.
5. **Given** configured nonzero usage, **When** the request succeeds, **Then** the completion reports the specified input, output, and cached values.
6. **Given** a scripted series of replies and a test-session identifier, **When** requests arrive, **Then** only that session advances through its script; a concurrent session has independent state.
7. **Given** request inspection is enabled, **When** the author reads the log, **Then** it shows the request's messages, tools, tool choice, token limit, and relevant headers; reset clears both the log and scripted state.
8. **Given** a test control for reasoning, multiple tool calls, malformed arguments, or a specified finish reason, **When** the request is handled, **Then** the chosen case is represented in the response without changing the default protocol.

---

### User Story 4 - Run and maintain the two projects reliably (Priority: P3)

A contributor installs Vizier's dependencies, starts the test server and Vizier locally, or uses the sample container setup. Automated checks test the server on its own and Vizier against that server. A maintainer can review and merge the server change before the Vizier change that depends on it.

**Why this priority**: A replacement that cannot be started or tested consistently would weaken the project's mandatory offline gate.

**Independent Test**: Follow documented setup from a fresh checkout, run both local and container paths, and run the server's conformance tests plus Vizier's existing dummyplug end-to-end checks.

**Acceptance Scenarios**:

1. **Given** a fresh recursive checkout, **When** a contributor follows the setup instructions, **Then** the test server is available without building it as part of Vizier's normal build.
2. **Given** the test server is unavailable, **When** a dummyplug agent tries to use it, **Then** Vizier reports an ordinary local-provider availability failure rather than silently using a private fallback.
3. **Given** the sample container setup, **When** both services start, **Then** Vizier can reach dummyplug at the configured service address.
4. **Given** a change to either project, **When** continuous checks run, **Then** standalone server checks and Vizier end-to-end checks run against the pinned server version.
5. **Given** separate changes to the server and Vizier, **When** maintainers review them, **Then** the dependency and merge order are documented and the Vizier change pins the reviewed server revision.

### Edge Cases

- Several trailing tool results are echoed in order; an unresolved tool name falls back to its call identifier.
- User text mixed with a separate context part, frontmatter, attachments, or a surrounding code fence is normalized before command matching.
- A malformed or unknown tool request remains a successful chat response containing an error explanation; only injected transport failures use error status codes.
- New controls cannot shadow `tools`, `context`, an exact tool name, or a legacy JSON tool request.
- One scripted test must not consume another test's steps; exhausted scripts return to documented default behavior.
- An interrupted or intentionally truncated stream must not be mistaken for a successful complete answer.
- Request inspection can contain private prompt content; only authorized local test callers may read or alter it.
- A stalled or missing server must produce a visible provider failure and must not prevent Vizier itself from starting.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Dummyplug MUST preserve every currently documented §1–§6 command, normalization, tool-call, tool-result, error-text, and default-response rule as its default behavior.
- **FR-002**: Existing agents using provider `dummyplug` and any model string MUST continue to work without a data migration when the test server is running.
- **FR-003**: Dummyplug MUST be runnable independently of Vizier and usable by a standard OpenAI-compatible chat client without a required API key.
- **FR-004**: The public chat surface MUST accept whole and streamed completions, tool definitions, tool results, text strings and text parts, tool choice, token limits, and optional streamed usage.
- **FR-005**: Every successful whole response MUST include a valid model identifier, finish reason, message, and usage; streamed responses MUST reassemble to the same content for the same request and seed.
- **FR-006**: Dummyplug MUST expose a model listing containing `dummyplug` and a readiness check.
- **FR-007**: Vizier MUST use its ordinary provider connection and response handling for dummyplug; there MUST be no remaining in-process dummyplug model or fallback.
- **FR-008**: Vizier MUST use a local default dummyplug address and allow the operator to override it for local, remote, and container setups.
- **FR-009**: Test authors MUST be able to request deterministic output, delays or holds, injected failures with a chosen count and retry timing, and nonzero usage without changing normal requests.
- **FR-010**: Test authors MUST be able to request reasoning content, scripted tool calls including multiple calls, malformed arguments, and a chosen finish reason.
- **FR-011**: Test authors MUST be able to configure isolated multi-request scenarios and inspect recent requests, including message content, tool definitions, tool choice, token limit, and relevant headers.
- **FR-012**: Test controls MUST be available as explicit message commands for manual use, per-request controls for SDK clients, and a control surface for scripted scenarios; new message commands MUST use a distinct `DUMMYPLUG` prefix.
- **FR-013**: The request log and scripted state MUST be bounded, held only for the running test-server process, resettable, and automatically cleared after inactivity.
- **FR-014**: The control and inspection surface MUST be limited to local callers by default and require a separate credential if intentionally exposed to a network; the public chat surface MUST remain keyless.
- **FR-015**: Delayed and streaming work MUST stop when the requesting client disconnects or cancels.
- **FR-016**: Legacy behavior that may call a tool despite `tool_choice: none` MUST remain the default; an explicit test control MAY make dummyplug honor tool choice.
- **FR-017**: Standalone conformance checks MUST cover model listing, whole and streamed text, tool calls, tool-result echo, and all legacy protocol cases without requiring Vizier.
- **FR-018**: Vizier's existing dummyplug quickstarts and end-to-end checks MUST run with the server; automated checks MUST start the server and test both projects together.
- **FR-019**: Documentation MUST explain local and container startup, the separate-server requirement, address override, new controls, all legacy commands, and the reviewed server-to-Vizier update order; stale tool examples MUST be corrected.
- **FR-020**: The normal Vizier build and test MUST remain independent of building the dummyplug server, and Vizier MUST remain runnable without the server when no dummyplug agent is used.

### Key Entities

- **Chat request**: The messages, tool definitions, model name, output controls, and optional test controls supplied by a client.
- **Chat completion**: The assistant's text or tool calls, finish reason, model, and usage, delivered whole or in pieces.
- **Test scenario**: An isolated sequence of configured replies or failures associated with one test-session identifier.
- **Request record**: A bounded, temporary inspection copy of what the server received.
- **Provider configuration**: The address Vizier uses to reach dummyplug; existing agent provider and model values remain unchanged.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of existing documented §1–§6 cases pass against the standalone server, with exact matches for deterministic cases and the documented response shape for random cases.
- **SC-002**: 100% of existing Vizier dummyplug quickstarts and end-to-end checks pass with the server running; no test requires a live model credential.
- **SC-003**: A stock OpenAI-compatible SDK completes model listing, whole chat, streamed chat, a tool call, and tool-result echo against dummyplug without Vizier.
- **SC-004**: For every seeded conformance case, reassembling streamed content yields exactly the same answer as a whole completion.
- **SC-005**: Automated checks demonstrate a temporary failure followed by success, a failed turn, nonzero usage, a cancellable hold, and two isolated parallel scenarios.
- **SC-006**: A contributor following the documented local setup can start both programs and complete a dummyplug conversation without editing an existing agent.
- **SC-007**: The normal Vizier build and tests complete without building dummyplug; Vizier starts successfully when the optional dummyplug server is absent and no dummyplug agent is used.

## Completion Gate

The feature is complete only when the following evidence is recorded against the *same* dummyplug commit pinned by Vizier. A passing replacement harness alone does not substitute for the existing checks named below.

| Gate | Required observable evidence |
|---|---|
| Legacy compatibility | Run the documented §1–§6 cases, including normalization, malformed commands, multiple trailing tool results, and `tool_choice: none`, against the standalone server. Compare deterministic output exactly and unseeded lorem by documented shape. |
| Existing Vizier flows | Run `scripts/client-tools-e2e.mjs` and `scripts/durable-turns-e2e.mjs` with a Vizier agent using the HTTP dummyplug provider. Run the existing dummyplug quickstart conversation, including a state-changing tool and verification of its result. Record any intentionally skipped restart step separately; a skip is not a pass for that step. |
| Provider failures and retries | Through Vizier, configure two HTTP failures followed by success. Observe three recorded requests for one turn and the final successful answer. Separately exhaust Vizier's retry policy and observe a failed turn while `/api/v1/ping` remains healthy. Verify nonzero and cached usage in the server response and in Vizier's recorded turn data. |
| Cancellation and streaming | While a request is held or delayed, disconnect the client before release; prove it yields no later completion and leaves no active response task. Abort an SSE client mid-stream and verify production stops. A deliberately truncated stream must omit both the final finish chunk and `[DONE]`. Check seeded whole/stream equivalence for text and tool calls. |
| Isolation and state bounds | Run two sessions concurrently and prove one cannot consume the other's steps. Exercise record count/byte eviction, session-cap rejection, reset, and inactivity expiry with observable API results, including a read after expiry without an intervening chat. Each inspection record must expose its timestamp and outcome while omitting authorization and control tokens. |
| Deployment and publication | Push the server commit to `vizier-lab/dummyplug` before pinning it in a Vizier PR. From a fresh recursive checkout, resolve the pinned gitlink, run the documented local path and Compose path, and reach dummyplug from Vizier at the service URL. CI must run standalone checks plus the Vizier gates against that gitlink. The normal Vizier build and a non-dummyplug startup must succeed without building or starting dummyplug. |

Record each command, commit SHA, result, and any environmental limitation in the implementation PR. A gate that has not run remains **unverified**, even if the corresponding code is present.

## Assumptions

- Issue #95 defines the legacy protocol and the required acceptance boundary; this spec prioritizes complete migration of current behavior before additive controls.
- Dummyplug is a test provider. Media-generation and embedding stubs are separate work.
- The server runs as a separate process during development and in container deployments. The end-to-end harness starts it automatically.
- The default address is local; operators can override it for other deployment layouts.
- Request inspection and scripted state are test data, not durable agent memory. They disappear after reset, inactivity, or process restart.
- The current random lorem-ipsum text cannot be compared byte-for-byte across unseeded runs; its formatting and dispatch rules are preserved, while seeded runs are reproducible.
- Tests and contracts, rather than a separate protocol-version number, govern compatibility of Vizier-specific input framing.
