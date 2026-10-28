# Feature Specification: AWS Bedrock Provider

**Feature Branch**: `034-aws-bedrock-provider`

**Created**: 2026-10-28

**Status**: Draft

**Input**: User description: "i want to add new provier for aws bedrock"

## Context

Vizier's provider layer speaks five wire protocols (OpenAI Chat, OpenAI Responses, Anthropic, Gemini and Ollama) and can reach any provider in the model catalogue that uses one of them. AWS Bedrock is in the catalogue with its models, context windows, capabilities and prices, but it is deliberately hidden from every provider and model picker because Vizier cannot speak Bedrock's API: a Bedrock request is not any of those five protocols, and a Bedrock endpoint is regional and requires AWS authentication rather than a single API key.

Today an operator who runs agents on AWS has no way to use Bedrock from Vizier. They cannot select it in Settings › Providers, an agent cannot be pointed at a Bedrock model, and the only workaround — a custom OpenAI-compatible gateway in front of Bedrock — loses the catalogue's models, capabilities and prices and requires running another service, which Vizier is built not to need.

This feature makes AWS Bedrock a first-class chat provider: one connection saved once in Settings › Providers, region included, then any Bedrock model chosen per agent like any other provider. Bedrock's unified Converse interface carries the conversations Vizier already builds (system prompts, multi-turn history, tool calls and results, images), so the agent loop, memory, sessions, scheduled runs, dreams and background jobs need no Bedrock-specific behaviour. Model limits, capabilities and prices keep coming from the catalogue, so context checkpoints, attachment handling, tool warnings and cost estimates work as they do for every other provider.

Scope is chat. Embeddings, image generation, speech, Batch, Agents, Guardrails, provisioned throughput, streaming and the AWS credential chain beyond static keys and Bedrock API keys are not part of this feature and are recorded as assumptions. The value is one connection and one bill for an operator already on AWS: an agent that uses Bedrock for text, tools and images, configured where every other provider is configured.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Connect AWS Bedrock once and talk to a Bedrock model (Priority: P1)

A person with an AWS account opens Settings › Providers, finds Amazon Bedrock, enters their AWS access key ID, secret access key and region, and saves. They create an agent, choose Amazon Bedrock as its provider, pick a Bedrock model from the model list, and send it a message. The agent answers using that model. From then on the connection is shared by every agent that selects Bedrock.

**Why this priority**: This is the whole feature in one slice: without a saved connection and a reply, nothing else matters. It also exercises the credential model, the regional endpoint, the protocol and the catalogue's model data end to end.

**Independent Test**: With only an AWS account and a Bedrock-enabled region, save the connection in Settings › Providers, create an agent with provider Amazon Bedrock and a listed model, and hold a multi-turn conversation. Repeat with the connection supplied entirely through environment variables, with no saved entry.

**Acceptance Scenarios**:

1. **Given** no Bedrock connection exists, **When** a person opens Settings › Providers and searches for Bedrock, **Then** Amazon Bedrock is listed with its logo and can be connected, instead of being hidden as unsupported.
2. **Given** a person enters a valid access key ID, secret access key and region, **When** they save and test the connection, **Then** the test reports success without starting an agent.
3. **Given** a saved Bedrock connection and an agent whose provider is Amazon Bedrock, **When** the agent is sent a message, **Then** it replies with a model response produced by Bedrock and the reply is stored in history like any other reply.
4. **Given** an agent on a Bedrock model, **When** the conversation continues across several turns, **Then** earlier turns, the agent's system prompt and its CORE document are all carried to the model, and the conversation stays coherent.
5. **Given** the connection comes from the environment rather than a saved entry, **When** the provider list is opened and an agent uses Bedrock, **Then** the provider is listed as coming from the environment variable that supplied it and the agent works without any saved credentials.
6. **Given** an access key ID but no secret access key, **When** the person tries to save or use the connection, **Then** the attempt is refused with a message naming the missing field, and no unauthenticated request is sent.
7. **Given** no region is set anywhere, **When** a Bedrock agent is started or tested, **Then** the failure names the region setting and the environment variables that can supply it, rather than guessing a region.

---

### User Story 2 - Bedrock models support the tools and images agents already use (Priority: P1)

An agent on a Bedrock model receives a request that needs tools — searching memory, running a script, calling another agent — and the model calls them. The results go back to the model and it finishes the turn. The same agent is sent a picture in the conversation and, because its model can see images, it answers about the picture. On a Bedrock model without vision, the picture is replaced by a short note and the agent still answers.

**Why this priority**: Tool calling is how Vizier agents do anything, and image input is part of a normal conversation on several channels. A Bedrock provider that could only exchange plain text would not be usable for real agents. It is independently testable and independently valuable.

**Independent Test**: Using a live AWS account, on an agent whose provider is Bedrock and whose model supports tool calling, ask a question that requires a tool (for example a memory search or a Python script), confirm the tool runs exactly once and the model's final reply uses its result. Then send an image to a vision-capable Bedrock model and to a text-only one and compare the two behaviours.

**Acceptance Scenarios**:

1. **Given** a Bedrock model that supports tool calling, **When** the model asks for a tool, **Then** the tool executes through the normal tool path with the agent's configuration, and its result is returned to the model in the same turn.
2. **Given** a model asks for several tools in one round, **When** the round runs, **Then** every call is executed and every result is returned together, and no call is executed twice.
3. **Given** an agent that restricts tool choice, **When** it runs on Bedrock, **Then** the restriction is honoured where the model supports it rather than being silently dropped.
4. **Given** a Bedrock model whose capabilities include image input, **When** the person sends an image, **Then** the model receives the image and the reply can refer to its contents.
5. **Given** a Bedrock model that cannot accept images, **When** a conversation containing images reaches it, **Then** each image is replaced by a text note in the request and the model still answers.
6. **Given** a tool call that fails or a tool result the model cannot use, **When** the round completes, **Then** the failure is reported to the model as a tool result and the turn ends with an explanation instead of crashing.
7. **Given** an agent on a Bedrock model with the catalogue's context window, **When** the conversation reaches the checkpoint threshold, **Then** the existing checkpoint and handover behaviour runs unchanged.

---

### User Story 3 - Credentials, health and cost behave like every other provider (Priority: P2)

A person rotates their AWS keys, removes the connection, or tries to use a region where their model is not enabled. Vizier tells them what is wrong, which agents are affected, and lets them recover by saving new credentials. Usage and estimated cost for Bedrock turns appear in the same usage views as every other provider.

**Why this priority**: A connection that fails silently or hides its cost is not operable. Rotation and failure reporting are routine on AWS, and usage accounting is already promised for every answered completion. It does not block the first two slices but must ship with the feature.

**Independent Test**: Run a turn, then replace the saved keys with invalid ones and confirm the next turn fails with an actionable message naming Bedrock and the credential fields, the agent is shown as failed rather than silently idle, and fixing the credentials makes the next turn succeed. Check the usage view for the successful turns.

**Acceptance Scenarios**:

1. **Given** a working Bedrock agent, **When** the saved credentials are replaced with invalid ones, **Then** the next turn fails with an actionable error, agent health reports the failure, and the agent recovers once valid credentials are saved.
2. **Given** a Bedrock agent, **When** the saved region is changed, **Then** the agents using Bedrock pick up the new region without affecting agents on other providers.
3. **Given** a Bedrock agent whose model is not enabled for the account or not offered in the configured region, **When** it tries to answer, **Then** the error names the model, the region and the likely cause, and the agent stays available for other work.
4. **Given** a Bedrock connection with an expired session token, **When** a turn runs, **Then** the provider's expiry is surfaced with the fix (a new session token or keys) rather than being reported as an unknown failure.
5. **Given** any answered Bedrock completion, **When** the usage report is opened, **Then** the turn appears with its input and output token counts and, when the catalogue prices the model, an estimated cost at the catalogue's rates.
6. **Given** a person removes the Bedrock connection, **When** an agent still configured for it runs, **Then** the error says the provider is not connected and points to Settings › Providers.
7. **Given** a Bedrock credential is saved, **When** any provider, agent or settings response is inspected, **Then** the secret access key, session token and Bedrock API key are never returned in clear, while the access key ID and region are shown so the person can confirm what is connected.

---

### User Story 4 - Bedrock API keys and cross-region model ids are accepted (Priority: P3)

A person whose organisation issues Bedrock API keys rather than IAM access keys connects with that single key and a region. Another person uses a cross-region inference profile id (`us.`, `eu.`, `apac.` or `global.` prefixed) copied from the AWS console, or types a model id the catalogue does not list yet, and it is used exactly as typed.

**Why this priority**: These are the two remaining ways real operators reach Bedrock and the two ways catalogue data is incomplete. They are cheap to support once the connection and protocol exist, but the feature is usable without them, so they rank last.

**Independent Test**: Save only a Bedrock API key with a region and hold a conversation; then configure an agent with a cross-region inference profile id and with an id the picker does not list, and confirm each is sent unchanged and either answers or returns the provider's error for that exact id.

**Acceptance Scenarios**:

1. **Given** only a Bedrock API key and a region are saved, **When** an agent runs, **Then** requests are authenticated with that key and succeed without any access key pair.
2. **Given** both an access key pair and a Bedrock API key are saved, **When** a request is made, **Then** the access key pair is used, consistently on every request, and the documented precedence explains why.
3. **Given** an agent configured with a cross-region inference profile id, **When** a turn runs, **Then** that id is sent unchanged and the configured region's endpoint is used.
4. **Given** a model id typed by hand that is not in the picker, **When** the agent runs, **Then** the id is sent unchanged and an invalid id produces the provider's error for that id rather than a silent substitution.

---

### Edge Cases

- The saved access key pair is incomplete (id without secret, or the reverse): saving is refused naming the missing field; an already-stored incomplete pair fails with the same message rather than sending a request.
- Both an access key pair and a Bedrock API key are present: the documented precedence applies on every request; rotating one while the other remains does not produce a silent mix.
- The region is missing, empty, or not a region string: the failure names the setting and the environment variables, and no request is attempted.
- The region is valid but the model is not offered there: the provider's error is surfaced with the model id and region; agents on other models are unaffected.
- The account has not been granted access to the model: the failure explains that model access may need enabling in AWS, names the model, and does not look like a credential error.
- An IAM policy lacks `bedrock:InvokeModel`: the permission error is surfaced with the missing action named where AWS returns it.
- A session token expires mid-turn: the failure names expiry, the turn ends cleanly with a saved history entry, and the next turn succeeds after new credentials are saved.
- Throttling or a provider 5xx: the existing retry and backoff policy applies, an eventual failure is reported as the provider's error, and the agent stays healthy.
- The request is rejected as too long or over the model's output limit: the provider's message is shown; the turn is not silently truncated or retried forever.
- An image is too large or in a format Bedrock refuses: the tool or turn reports the provider's explanation, and already produced results in that turn are kept.
- A PDF or another document attachment is sent to a model that cannot read it: it is replaced by a note like any unsupported attachment, never sent as a broken image.
- The picker's catalogue lists a model retired upstream, or lags behind a new release: the listed or typed id is still attempted and the provider's answer decides.
- Two agents share the Bedrock connection: one saved credential serves both; changing it restarts both and leaves agents on other providers untouched.
- Two Bedrock regions are needed at once: this is out of scope for the single connection; the limitation is documented rather than silently choosing one.
- The account uses AWS SSO, an instance profile or an assumed role with no static keys: the connection cannot be saved from those sources; the error names the supported credential forms rather than pretending to support the chain.
- A turn is aborted or the agent restarts while a Bedrock request is in flight: no partial assistant message is saved, and the existing stop/resume rules apply.
- A model returns a response with blocks Vizier does not map (for example reasoning or citation blocks): the response is still usable; unsupported blocks never fail the turn.
- The Bedrock connection is removed while agents use it: each affected agent reports the missing provider, and no request is attempted with half-deleted credentials.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `amazon-bedrock` MUST be a selectable chat provider in Settings › Providers, in the agent editor's provider and model pickers, and through the HTTP API, shown with its catalogue name, logo and model list instead of being hidden as unsupported.
- **FR-002**: A Bedrock connection MUST accept an AWS access key ID, secret access key, optional session token and region, and MUST accept a Bedrock API key with a region as an alternative way to authenticate.
- **FR-003**: The region MUST be required for every Bedrock connection, taken from the saved connection, then `AWS_REGION`, then `AWS_DEFAULT_REGION`; when none is present the failure MUST name the setting and the environment variables that supply it, and MUST NOT guess a region.
- **FR-004**: Bedrock MUST use the saved connection when present and otherwise credentials found in the environment (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN`, `AWS_BEARER_TOKEN_BEDROCK`), following the existing provider precedence and reporting the source in the provider list.
- **FR-005**: An access key pair with one half missing MUST be refused with a message naming the missing field, and no Bedrock request MUST ever be sent without complete authentication.
- **FR-006**: When both an access key pair and a Bedrock API key are present, exactly one documented authentication method MUST be used for every request; the rule MUST be stated in the documentation and not vary per call.
- **FR-007**: Bedrock requests MUST go to the regional Bedrock endpoint derived from the configured region, and a Bedrock connection MUST NOT accept a `base_url` override; an operator who needs another address uses a custom provider.
- **FR-008**: Bedrock models MUST be listed from the existing model catalogue with their identifiers, readable names, context windows, output limits, capabilities and prices, and choosing one MUST send that identifier unchanged, including cross-region inference profile prefixes.
- **FR-009**: A model identifier that is absent from the picker MUST be accepted when typed or supplied through the API, stored exactly as entered and sent unchanged.
- **FR-010**: Chat MUST use Bedrock's unified conversation interface (Converse), with Vizier's system prompt, full multi-turn history, tool definitions, tool results and assistant text translated to its request shape and its response translated back into Vizier's message parts.
- **FR-011**: Tool calling MUST work end to end: tool definitions offered to the model, every requested call executed once through the normal tool path, results returned to the model, several calls per round supported, and the agent's tool-choice restriction honoured where the model supports it.
- **FR-012**: Image input MUST be sent to models whose catalogue capabilities include it, in a form Bedrock accepts; for models that cannot accept images, each image MUST be replaced by the existing text note rather than failing the request.
- **FR-013**: Every answered Bedrock completion MUST record its input and output token counts in usage accounting, priced at the catalogue's rates when the model is priced, exactly as other providers' completions are.
- **FR-014**: The context window, output limit and capabilities MUST come from the same resolution order as other providers (agent override, then catalogue), and the source of the effective context window MUST appear in agent health.
- **FR-015**: Bedrock's failures MUST be surfaced as actionable errors that preserve the provider's error code and message: missing or incomplete credentials, missing region, model not available in the region, model access not enabled, missing IAM permission, expired session token, throttling, validation errors, service unavailability and timeouts MUST each read differently enough to act on.
- **FR-016**: A failed Bedrock request MUST NOT crash the agent, lose results already produced in the turn, or leave a half-written assistant message; the agent MUST stay available and its health MUST report the failure until it recovers.
- **FR-017**: Retries MUST follow the existing provider policy: throttling and retryable server failures are retried with backoff (honouring the provider's retry hint where present), and an exhausted retry budget ends the turn with the provider's error.
- **FR-018**: The existing Test action for a provider MUST verify a Bedrock connection without starting an agent, report success or the provider's failure with a readable reason, and keep the saved credentials either way.
- **FR-019**: Saving, changing or removing Bedrock credentials or the region MUST restart exactly the agents that use Bedrock, through the existing provider-change path, and MUST NOT affect agents on other providers; the next turn succeeds without restarting Vizier.
- **FR-020**: Secret access keys, session tokens and Bedrock API keys MUST be encrypted at rest, MUST never be returned by any API, screen, log line, error message or telemetry attribute, and MUST be write-only after saving; the access key ID and region MAY be shown so the person can identify the connection.
- **FR-021**: Bedrock MUST work everywhere any other chat provider works without special cases: ordinary chat on every channel, the dream cycle, scheduled tasks, subagents, background jobs, inter-agent messages, handovers and checkpoints, and conversation history that predates the provider switch.
- **FR-022**: Startup, provider and agent builds, the scheduler and dreams MUST NOT make any AWS request; requests happen only for a person-triggered test or an agent turn, and the catalogue's refresh rules are unchanged.
- **FR-023**: Existing providers, agents and stored configurations MUST behave exactly as before; no migration is required, and an agent on another provider MUST NOT be restarted or changed by this feature.
- **FR-024**: The documentation MUST cover connecting Bedrock, the credential and region fields and their environment variables, which authentication method takes precedence, the required AWS permissions, the model-access prerequisite, the regional and cross-region model-id behaviour, and the single-region limitation; the encryption documentation MUST list the new protected fields.
- **FR-025**: Bedrock MUST appear in the provider list as a connectable provider (not as unsupported) and MUST be reachable through the same list, test, save and delete API surface as other providers.

### Key Entities

- **Bedrock connection**: the one saved credential entry for the provider, holding an access key ID, secret access key, optional session token, optional Bedrock API key and region, plus where the values came from (saved or an environment variable). Secrets are write-only and encrypted; the key ID and region are identifiers.
- **Bedrock model entry**: a catalogue model of the Bedrock provider with its identifier (possibly a cross-region inference profile), readable name, context window, output limit, capabilities (tool calling, image input) and price; the picker's source and the agent's selected value.
- **Agent provider selection**: an agent's provider id plus model id, with optional context-window and output overrides, resolved at build time into the effective endpoint, credential and limits.
- **Bedrock conversation request/response**: the translated turn — system prompt, messages, tool definitions and results, images — and the model's answer parts, stop reason and token usage.
- **Usage record**: one row per answered completion with provider, model, input and output tokens, activity and session, priced later from the catalogue.
- **Provider health**: the state an affected agent reports when its Bedrock request fails, including the last error and, when healthy, the effective context window and its source.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A person with an AWS account, a region and credentials can connect Bedrock and get a first agent reply in under 3 minutes, with no step outside Settings › Providers and the agent editor.
- **SC-002**: 100% of the verification turns in the matrix — first turn, multi-turn, system prompt and CORE, tool call, several tool calls, tool result, image to a vision model, image to a text-only model, and a checkpointed long conversation — complete with the expected reply or the expected replacement behaviour.
- **SC-003**: In 100% of missing-credential, incomplete-pair, missing-region and wrong-region trials, the person sees an actionable message naming the setting (or environment variable) to fix, and no unauthenticated request is sent.
- **SC-004**: In 100% of checks, no secret access key, session token or Bedrock API key appears in an API response, a WebUI screen, a log line, an error message or a telemetry attribute; the access key ID and region remain visible.
- **SC-005**: 100% of tool-using turns execute each requested call exactly once, return every result to the model, and end with the model's final answer rather than a provider error.
- **SC-006**: A vision-capable Bedrock model answers correctly about an image in 100% of runs, and a text-only Bedrock model answers with the image replaced by a note in 100% of runs.
- **SC-007**: 100% of answered Bedrock completions appear in the usage views with non-zero token counts where the provider reports them, and their estimated cost matches the catalogue's rate for the model; models with no catalogue price show no invented cost.
- **SC-008**: After invalid credentials are replaced with valid ones, the next turn succeeds with no Vizier restart, and agents on other providers are never restarted by a Bedrock credential change.
- **SC-009**: 100% of the existing provider and agent regression checks still pass; no existing stored configuration changes behaviour and no migration runs.
- **SC-010**: Zero AWS requests are made at startup, during agent builds, by the scheduler or by the dream cycle; every request is attributable to a person-triggered test or an agent turn.
- **SC-011**: A person finds Amazon Bedrock in Settings › Providers and a Bedrock model in an agent's picker on the first attempt, without consulting external documentation, in 100% of usability trials.
- **SC-012**: In 100% of injected provider failures (throttling, model not enabled, permission denied, expired token, invalid model id, oversized request), the failure is reported with the provider's explanation, the agent remains able to run a later turn, and no partial assistant message is stored.

## Assumptions

- The provider's identifier is the catalogue's `amazon-bedrock`, displayed as **Amazon Bedrock**; its models, limits, capabilities and prices continue to come from the existing catalogue (models.dev) rather than a new listing service.
- The protocol is Bedrock's unified Converse interface for chat, which covers text, system prompts, multi-turn messages, tool use and image input across the model families Bedrock offers. A single protocol path serves every Bedrock model rather than one implementation per model family.
- Two authentication methods are supported: an IAM access key ID with secret access key and optional session token (signed requests), and a Bedrock API key sent as a bearer token. The AWS credential chain — shared profiles, SSO, instance profiles, ECS task roles, assumed roles and credential-process plugins — is out of scope; operators using it supply keys through the environment or a sidecar, or use a custom provider.
- When both an access key pair and a Bedrock API key are present, the access key pair is used, because it is the production-grade method and the one whose rotation is explicit; the Bedrock API key is the fallback. This precedence is documented and identical on every request.
- One Bedrock region is configured per installation, shared by all agents on the provider. Multiple regions at once, per-agent regions and automatic failover between regions are out of scope; cross-region inference profile identifiers remain the way to reach multiple regions from one configured region.
- Model availability and model access are the operator's responsibility on AWS: a model may be listed in the catalogue but not enabled for the account or offered in the chosen region, and the provider's error is authoritative. Vizier does not silently substitute another model.
- Streaming responses are not used: the turn receives the complete response, as with every provider except the one that requires streaming. The `InvokeModelWithResponseStream` action is not required.
- Extended thinking, reasoning-content display, prompt caching, structured outputs, Guardrails, provisioned throughput, Batch inference, Bedrock Agents and Bedrock Knowledge Bases are out of scope. Responses containing blocks Vizier does not map must still produce a usable turn.
- Embeddings, text-to-speech, speech-to-text and image generation through Bedrock (Titan/Nova models) are out of scope; those capabilities keep their current providers.
- The connection is opt-in: a fresh install makes no AWS call and needs no AWS configuration, preserving config-less startup. Only a person's test or an agent turn reaches AWS.
- The required AWS permissions are the model-invocation permissions (and, if the connection test uses a read-only model description, the corresponding read permission); the exact policy is documented in the same change.
- Prices come from the catalogue and may be missing for a model or become stale; usage without a price is still recorded and shown without an invented cost estimate.
- The existing retry policy, timeouts and error classification are reused rather than reinvented for Bedrock; only Bedrock-specific error translation is added.
- The WebUI connect form is generic over the provider's declared fields, so the new fields appear without redesigning Settings › Providers; the model picker already lists catalogue models and needs only the provider to become selectable.
- Verification uses a live AWS account because no offline server speaks Bedrock; those steps are marked as live-provider steps in the quickstart, and the dummyplug-based checks cover the parts that do not need AWS.
