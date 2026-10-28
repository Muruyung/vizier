# Feature Specification: AWS Bedrock Providers

**Feature Branch**: `034-aws-bedrock-provider`

**Created**: 2026-10-28

**Status**: Draft

**Input**: User description: "i want to add new provier for aws bedrock" (revised: two Bedrock providers, IAM keys and SSO through the AWS CLI; chat, embeddings and media included)

## Context

Vizier's provider layer speaks five wire protocols (OpenAI Chat, OpenAI Responses, Anthropic, Gemini and Ollama) and can reach any provider in the model catalogue that uses one of them. AWS Bedrock is in the catalogue with its chat models, context windows, capabilities and prices, but it is deliberately hidden from every provider and model picker because Vizier cannot speak Bedrock's API: a Bedrock request is not any of those five protocols, and a Bedrock endpoint is regional and requires AWS request signing rather than a single API key.

Today an operator who runs agents on AWS has no way to use Bedrock from Vizier. They cannot select it in Settings › Providers, an agent cannot be pointed at a Bedrock model or use Bedrock for memory embeddings or image generation, and the only workaround — a custom OpenAI-compatible gateway in front of Bedrock — loses the catalogue's models, capabilities and prices and requires running another service, which Vizier is built not to need.

AWS organisations issue two different kinds of credentials: long-lived IAM access keys, and short-lived role credentials obtained through AWS IAM Identity Center (SSO) single sign-on. The first is a pair of values Vizier can store and sign with. The second is already solved by the AWS CLI: the person runs `aws sso login --profile <profile>`, the CLI keeps the SSO session and resolves temporary role credentials for that profile, and any tool can obtain them through the CLI. Vizier therefore offers **two provider entries** — Amazon Bedrock (IAM) and Amazon Bedrock (SSO) — where the SSO entry names an AWS CLI profile and asks the CLI for that profile's credentials, storing no SSO token of its own. This keeps one sign-in workflow for everyone already using the AWS CLI and avoids duplicating AWS's SSO implementation inside Vizier.

One IAM connection or one SSO connection (or both) is saved once in Settings › Providers, region included, then each capability chooses one of them and its model like any other provider. Bedrock's unified Converse interface carries the conversations Vizier already builds (system prompts, multi-turn history, tool calls and results, images), and its embedding and image models receive the same prompts and text the existing memory indexer and image tool produce. Model limits, capabilities and prices keep coming from the catalogue, so context checkpoints, attachment handling, tool warnings and cost estimates work as they do for every other provider.

Speech is the one media capability Bedrock does not offer as a batch API (its speech model is real-time bidirectional), so text-to-speech and speech-to-text keep their current providers. Image-to-image, video generation, multiple profiles in one connection, browser sign-in started by Vizier, and reading the AWS CLI's caches directly are not part of this feature and are recorded as assumptions. The value is one connection and one bill for an operator already on AWS: agents that talk, remember and draw through Bedrock, configured where every other provider is configured.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Connect AWS Bedrock with IAM keys and talk to a Bedrock model (Priority: P1)

A person with an AWS account opens Settings › Providers, finds Amazon Bedrock (IAM), enters their AWS access key ID, secret access key and region, and saves. They create an agent, choose Amazon Bedrock (IAM) as its provider, pick a Bedrock model from the model list, and send it a message. The agent answers using that model. From then on the connection is shared by every agent that selects it.

**Why this priority**: This is the core of the feature: without a saved connection and a reply, nothing else matters. It exercises the credential model, the regional endpoint, the protocol and the catalogue's model data end to end.

**Independent Test**: With only an AWS account and a Bedrock-enabled region, save the IAM connection in Settings › Providers, create an agent with that provider and a listed model, and hold a multi-turn conversation. Repeat with the connection supplied entirely through environment variables, with no saved entry.

**Acceptance Scenarios**:

1. **Given** no Bedrock connection exists, **When** a person opens Settings › Providers and searches for Bedrock, **Then** Amazon Bedrock (IAM) is listed with its logo and can be connected, instead of being hidden as unsupported.
2. **Given** a person enters a valid access key ID, secret access key and region, **When** they save and test the connection, **Then** the test reports success without starting an agent.
3. **Given** a saved IAM connection and an agent whose provider is Amazon Bedrock (IAM), **When** the agent is sent a message, **Then** it replies with a model response produced by Bedrock and the reply is stored in history like any other reply.
4. **Given** an agent on a Bedrock model, **When** the conversation continues across several turns, **Then** earlier turns, the agent's system prompt and its CORE document are all carried to the model, and the conversation stays coherent.
5. **Given** the IAM connection comes from the environment rather than a saved entry, **When** the provider list is opened and an agent uses it, **Then** the provider is listed as coming from the environment variable that supplied it and the agent works without any saved credentials.
6. **Given** an access key ID but no secret access key, **When** the person tries to save or use the connection, **Then** the attempt is refused with a message naming the missing field, and no unauthenticated request is sent.
7. **Given** no region is set anywhere, **When** an IAM Bedrock agent is started or tested, **Then** the failure names the region setting and the environment variables that can supply it, rather than guessing a region.
8. **Given** an agent configured with a cross-region inference profile id (`us.`, `eu.`, `apac.` or `global.` prefixed), **When** a turn runs, **Then** that id is sent unchanged to the configured region's endpoint.
9. **Given** a model id typed by hand that the picker does not list, **When** the agent runs, **Then** the id is sent unchanged and an invalid id produces the provider's error for that id rather than a silent substitution.

---

### User Story 2 - Bedrock models support the tools and images agents already use (Priority: P1)

An agent on a Bedrock model receives a request that needs tools — searching memory, running a script, calling another agent — and the model calls them. The results go back to the model and it finishes the turn. The same agent is sent a picture in the conversation and, because its model can see images, it answers about the picture. On a Bedrock model without vision, the picture is replaced by a short note and the agent still answers. This works identically through the IAM and the SSO connection.

**Why this priority**: Tool calling is how Vizier agents do anything, and image input is part of a normal conversation on several channels. A Bedrock provider that could only exchange plain text would not be usable for real agents. It is independently testable and independently valuable.

**Independent Test**: Using a live AWS account, on an agent whose provider is either Bedrock connection and whose model supports tool calling, ask a question that requires a tool (for example a memory search or a Python script), confirm the tool runs exactly once and the model's final reply uses its result. Then send an image to a vision-capable Bedrock model and to a text-only one and compare the two behaviours.

**Acceptance Scenarios**:

1. **Given** a Bedrock model that supports tool calling, **When** the model asks for a tool, **Then** the tool executes through the normal tool path with the agent's configuration, and its result is returned to the model in the same turn.
2. **Given** a model asks for several tools in one round, **When** the round runs, **Then** every call is executed and every result is returned together, and no call is executed twice.
3. **Given** an agent that restricts tool choice, **When** it runs on Bedrock, **Then** the restriction is honoured where the model supports it rather than being silently dropped.
4. **Given** a Bedrock model whose capabilities include image input, **When** the person sends an image, **Then** the model receives the image and the reply can refer to its contents.
5. **Given** a Bedrock model that cannot accept images, **When** a conversation containing images reaches it, **Then** each image is replaced by a text note in the request and the model still answers.
6. **Given** a tool call that fails or a tool result the model cannot use, **When** the round completes, **Then** the failure is reported to the model as a tool result and the turn ends with an explanation instead of crashing.
7. **Given** an agent on a Bedrock model with the catalogue's context window, **When** the conversation reaches the checkpoint threshold, **Then** the existing checkpoint and handover behaviour runs unchanged.

---

### User Story 3 - Use an SSO profile already signed in through the AWS CLI (Priority: P2)

A person whose organisation uses AWS IAM Identity Center signs in on the machine running Vizier with `aws sso login --profile work`. In Settings › Providers they open Amazon Bedrock (SSO), enter the profile name `work` (and, if the profile's own region is not the one Bedrock should be called in, a region), and save. Vizier asks the AWS CLI for that profile's credentials and the agent answers. Hours later the CLI's SSO session expires; the agent's failure tells them to run `aws sso login --profile work`. They run it, and the next turn works without re-saving the connection or restarting Vizier. Vizier never stores an SSO token.

**Why this priority**: Organisations that use IAM Identity Center often prohibit long-lived access keys, so without an SSO path they cannot use Bedrock at all. It is a separate connection from IAM because it depends on the AWS CLI and a profile rather than stored keys, and it is independently testable once the protocol exists.

**Independent Test**: On a machine with the AWS CLI v2 installed, run `aws sso login --profile <profile>`, save the SSO connection with that profile, create an agent with provider Amazon Bedrock (SSO) and hold a conversation. Let the CLI session expire, confirm the failure names the login command, run it, and confirm the next turn succeeds without editing the connection or restarting Vizier.

**Acceptance Scenarios**:

1. **Given** no SSO connection exists, **When** a person opens its connect form, **Then** it asks for an AWS CLI profile name and an optional Bedrock region, and explains that the profile must be signed in with `aws sso login`.
2. **Given** a profile already signed in through the AWS CLI, **When** the person saves and tests the connection, **Then** Vizier resolves the profile's temporary credentials through the CLI and the test reports success.
3. **Given** a saved SSO connection, **When** an agent whose provider is Amazon Bedrock (SSO) is sent a message, **Then** it replies using the profile's role credentials, and the same connection can be chosen for embeddings and image generation.
4. **Given** the CLI's SSO session has expired, **When** a turn runs, **Then** the failure names the profile and the exact `aws sso login --profile <profile>` command to run, agent health reports it, and the next turn after running that command succeeds without restarting Vizier.
5. **Given** the AWS CLI is not installed, is older than the version that can export credentials for a profile, or is not on the executable path, **When** the connection is tested or used, **Then** the failure names the CLI as the cause and suggests installing/upgrading it or using the IAM connection instead.
6. **Given** a profile name that does not exist, **When** the connection is tested or used, **Then** the failure names the profile and the AWS config file it was looked for in.
7. **Given** the profile's role lacks permission to invoke Bedrock, **When** the connection is tested or a turn runs, **Then** the failure names the account, role and missing permission.
8. **Given** the profile field is left empty, **When** the connection is used, **Then** the AWS CLI's default profile resolution applies (`AWS_PROFILE`, or `default`), and that is documented.
9. **Given** Vizier restarts while the CLI session is still valid, **When** an agent runs, **Then** the connection works with no reconfiguration, because nothing about the SSO session is stored by Vizier.
10. **Given** both an IAM and an SSO Bedrock connection are saved, **When** agents select either, **Then** each works independently and changing, expiring or removing one does not affect the other.
11. **Given** the profile is changed, **When** the connection is saved, **Then** the old profile's cached credentials are discarded and the new profile is resolved on the next test or turn.

---

### User Story 4 - Memory search is embedded through either Bedrock connection (Priority: P2)

A person opens an agent's embedding settings, chooses Amazon Bedrock (IAM) or Amazon Bedrock (SSO), and leaves the model empty, or picks a Bedrock embedding model explicitly. The agent saves memories as usual; each passage is embedded through Bedrock and stored in the memory index. A later `memory_search` finds the saved passage. No second setup is needed for either connection.

**Why this priority**: Memory is the feature that makes an agent continuous, and embeddings decide whether recall works at all. A Bedrock connection that could chat but not index memory would force the operator to keep a second vendor just for embeddings. It is independently testable and independent of image generation.

**Independent Test**: With only a Bedrock connection, set an agent's embedding provider to that connection with an empty model, save a memory containing a distinctive phrase, then search for that phrase and confirm the passage is returned. Repeat with an explicitly chosen Bedrock embedding model and with the other Bedrock connection.

**Acceptance Scenarios**:

1. **Given** an agent whose embedding provider is either Bedrock connection and whose model is empty, **When** the agent is built, **Then** it uses the documented default Bedrock embedding model and starts successfully.
2. **Given** a Bedrock embedding provider, **When** a memory document is saved, **Then** its passages are embedded through Bedrock and stored in the memory index without failing the save.
3. **Given** memories embedded through Bedrock, **When** `memory_search` runs, **Then** it returns the passages that match, with the same addresses, limits and behaviour as any other embedding provider.
4. **Given** an explicit Bedrock embedding model, **When** it is saved and the agent rebuilt, **Then** that model is used and its returned vectors are stored.
5. **Given** the chosen connection is missing credentials or a region, or its CLI session has expired, **When** an agent with Bedrock embeddings is built, **Then** the build fails with an actionable error naming Bedrock and the setting or command to fix, rather than starting with silent recall.
6. **Given** a model id that is not a supported Bedrock embedding model, **When** it is used, **Then** the failure names the model and the supported model families, and the agent does not present broken recall as working.
7. **Given** an agent using local embeddings or another provider's embeddings, **When** this feature ships, **Then** it keeps working exactly as before with no reindex and no configuration change.

---

### User Story 5 - Images are generated through either Bedrock connection (Priority: P2)

A person enables image generation on an agent, chooses Amazon Bedrock (IAM) or Amazon Bedrock (SSO), leaves the model empty or picks a Bedrock image model, and asks for a picture. The agent generates it through the chosen connection and delivers it in the conversation, where it can be opened and downloaded like any other generated image.

**Why this priority**: Drawing is the other media capability Bedrock offers in batches, and it is independently valuable: an operator on AWS can stop paying for a separate image vendor. It is separate from chat and embeddings and can ship on its own.

**Independent Test**: With only a Bedrock connection, enable image generation with that provider and no model, ask the agent for an image, and confirm a valid image file appears in the conversation's files and is displayed. Repeat with an explicit model and a size, and with the other Bedrock connection.

**Acceptance Scenarios**:

1. **Given** an agent with image generation enabled for either Bedrock connection and no model set, **When** the agent is asked to draw something, **Then** it uses the documented default Bedrock image model, saves a valid image in the conversation's files and delivers it as an attachment.
2. **Given** an explicit model and size are configured, **When** an image is generated, **Then** that model is used and the requested size is honoured where the model supports it.
3. **Given** no size is configured, **When** an image is generated, **Then** the documented default size is used and the returned image opens in a standard viewer.
4. **Given** Bedrock returns the image as PNG or JPEG, **When** it is saved, **Then** the file keeps the media type the provider actually returned, with a matching extension, and is not mislabeled.
5. **Given** the selected model is not a supported Bedrock image model, **When** generation is attempted, **Then** the tool call fails with a message naming the model and the supported families, and other tools in the turn are unaffected.
6. **Given** Bedrock blocks the prompt or returns no image, **When** generation is attempted, **Then** the tool call fails with the provider's explanation and no empty or broken file is saved.
7. **Given** the chosen connection is missing credentials or a region, or its CLI session has expired, **When** an agent with Bedrock image generation is built, **Then** it reports an actionable error naming Bedrock and the setting or command to fix.

---

### User Story 6 - Credentials, sign-in state, health and cost behave like every other provider (Priority: P2)

A person rotates their AWS keys, changes the region, re-runs `aws sso login`, removes a connection, or tries to use a region where their model is not enabled. Vizier tells them what is wrong, which agents are affected, and lets them recover by saving new credentials or signing in again with the CLI. Usage and estimated cost for Bedrock chat turns appear in the same usage views as every other provider, and each connection's change applies to chat, embeddings and image generation together.

**Why this priority**: A connection that fails silently or hides its cost is not operable. Rotation and failure reporting are routine on AWS, and usage accounting is already promised for every answered completion. It does not block the first five slices but must ship with the feature.

**Independent Test**: Run a chat turn on each connection, then make the credentials invalid and confirm the next turn fails with an actionable message naming Bedrock and the credential or CLI state, the agent is shown as failed rather than silently idle, and fixing it makes the next turn succeed. Check the usage view for the successful turns and confirm embeddings and image calls are treated like other non-chat media.

**Acceptance Scenarios**:

1. **Given** a working IAM Bedrock agent, **When** the saved credentials are replaced with invalid ones, **Then** the next turn fails with an actionable error, agent health reports the failure, and the agent recovers once valid credentials are saved.
2. **Given** a working SSO Bedrock agent whose CLI session expired, **When** the next turn runs, **Then** the failure names the login command and profile, and running it restores turns without a Vizier restart or a connection change.
3. **Given** a Bedrock agent, **When** its connection's region is changed, **Then** chat, embeddings and image generation on that connection use the new region without affecting agents on the other Bedrock connection or on other providers.
4. **Given** a Bedrock agent whose model is not enabled for the account or not offered in the configured region, **When** it tries to answer, **Then** the error names the model, the region and the likely cause, and the agent stays available for other work.
5. **Given** an IAM connection with an expired session token, **When** a turn runs, **Then** the provider's expiry is surfaced with the fix (a new session token or keys) rather than being reported as an unknown failure.
6. **Given** a profile whose SSO role credentials expire during a long-running session, **When** the next turn runs, **Then** Vizier resolves fresh credentials through the CLI before the request, so the turn succeeds without human action while the CLI session is still valid.
7. **Given** any answered Bedrock chat completion, **When** the usage report is opened, **Then** the turn appears with its input and output token counts and, when the catalogue prices the model, an estimated cost at the catalogue's rates.
8. **Given** a Bedrock embedding or image call, **When** usage is inspected, **Then** it is treated exactly like the existing embedding and image calls of other providers and no new cost is invented.
9. **Given** a person removes either Bedrock connection, **When** an agent still configured for it runs, **Then** the error says the provider is not connected and points to Settings › Providers.
10. **Given** a Bedrock credential or SSO profile is saved, **When** any provider, agent or settings response is inspected, **Then** the IAM secret access key, session token and any CLI-resolved temporary credentials are never returned in clear, while the IAM access key ID, or the SSO profile name and region, are shown so the person can confirm what is connected.

---

### Edge Cases

**IAM**

- The saved access key pair is incomplete (id without secret, or the reverse): saving is refused naming the missing field; an already-stored incomplete pair fails with the same message rather than sending a request.
- Only a Bedrock API key (`AWS_BEARER_TOKEN_BEDROCK`) is supplied, with no access key pair: the failure says that Bedrock API keys are not supported yet and names the access-key variables, rather than reporting a generic missing key.
- The region is missing, empty, or not a region string: the failure names the setting and the environment variables, and no request is attempted.
- A session token expires mid-turn: the failure names expiry, the turn ends cleanly with a saved history entry, and the next turn succeeds after new credentials are saved.

**SSO through the AWS CLI**

- `aws sso login --profile <profile>` has never been run: the CLI's own "session expired, run aws sso login" message is surfaced with the exact command and profile, and no request is attempted.
- The CLI session expires between turns: fresh credentials are resolved before the next request; when the CLI refuses, the actionable re-login error is shown and the turn is not attempted with expired credentials.
- The AWS CLI is missing, not on `PATH`, not executable, or too old to export credentials for a profile: the failure names the CLI as the cause and suggests installing or upgrading it, or using the IAM connection.
- The CLI hangs, crashes, or prints malformed JSON: the attempt fails with a readable explanation after a bounded wait, no partial credential is used, and a later attempt can retry.
- The profile does not exist, or its name has surrounding spaces: the failure names the profile and the AWS config file, and the value is passed as a single argument, never through a shell.
- The profile is signed in but has no role for Bedrock's account, or the role cannot be assumed: the CLI's error is surfaced with the account and role named.
- The role lacks `bedrock:InvokeModel`: the permission error is surfaced with the missing action named where AWS returns it.
- The profile points at a non-SSO credential source (static keys in the profile, an assumed role, a credential process): the CLI resolves it and it works, but the documentation calls the SSO profile the intended use.
- The CLI returns credentials without an expiry: they are used for that request and re-resolved on the next one rather than cached indefinitely.
- Several agents resolve credentials at the same moment when they expire: one CLI resolution happens and the others use its result, so the CLI is not spawned repeatedly.
- Vizier restarts: nothing about the SSO session is stored, so the first use resolves through the CLI again; if the CLI session is still valid it works with no re-login, otherwise the actionable re-login error is shown.
- The profile or region is changed: the old profile's cached credentials are discarded before the new one is resolved.
- Both Bedrock connections are saved with different regions: each agent uses the connection it selected, and changing one does not move the other.
- The SSO profile and the IAM connection point at the same AWS account: they remain independent connections; an expired CLI session never falls back to the IAM keys or the other way round.

**Chat, embeddings and images**

- The region is valid but the model is not offered there: the provider's error is surfaced with the model id and region; agents on other models are unaffected.
- The account has not been granted access to the model: the failure explains that model access may need enabling in AWS, names the model, and does not look like a credential error.
- Throttling or a provider 5xx: the existing retry and backoff policy applies, an eventual failure is reported as the provider's error, and the agent stays healthy.
- The request is rejected as too long or over the model's output limit: the provider's message is shown; the turn is not silently truncated or retried forever.
- An image is too large or in a format Bedrock refuses: the tool or turn reports the provider's explanation, and already produced results in that turn are kept.
- A PDF or another document attachment is sent to a model that cannot read it: it is replaced by a note like any unsupported attachment, never sent as a broken image.
- The picker's catalogue lists a model retired upstream, or lags behind a new release: the listed or typed id is still attempted and the provider's answer decides.
- The embedding model returns vectors whose size differs from the agent's existing memory index: the incompatibility is reported clearly and no vectors of a different size are written into the index.
- The embedding model is a chat model id, or the image model is an embedding id: the failure names the model and the supported families instead of sending a malformed request.
- An embedding fails while a memory document is being saved: the save still succeeds, following the existing behaviour, and the failure is logged rather than silently losing the document.
- An image request returns more than one image: the first is saved; the rest are ignored, matching the existing single-result contract of the image tool.
- The prompt is blocked by Bedrock's content filter: the tool call reports the provider's explanation and saves no file.
- Two agents share one Bedrock connection, one using chat and one embeddings: one saved connection serves both; changing it restarts both and leaves agents on the other connection and on other providers untouched.
- Two Bedrock regions are needed on the same connection: this is out of scope; the limitation is documented rather than silently choosing one.
- A turn is aborted or the agent restarts while a Bedrock request is in flight: no partial assistant message is saved, and the existing stop/resume rules apply.
- A model returns a response with blocks Vizier does not map (for example reasoning or citation blocks): the response is still usable; unsupported blocks never fail the turn.
- A connection is removed while agents use it: each affected agent reports the missing provider, and no request is attempted with half-deleted credentials.

## Requirements *(mandatory)*

### Functional Requirements

**Providers and connections**

- **FR-001**: Vizier MUST offer two distinct Bedrock provider entries — **Amazon Bedrock (IAM)** and **Amazon Bedrock (SSO)** — each selectable for chat, embeddings and image generation in Settings › Providers, in the agent editor's provider and model pickers, and through the HTTP API, shown with the Bedrock name and logo instead of being hidden as unsupported.
- **FR-002**: Both entries MUST present the same Bedrock chat model list (from the catalogue) and the same supported embedding and image model families; only their authentication and connection fields differ.
- **FR-003**: The IAM entry MUST accept an AWS access key ID, secret access key, optional session token and region. A Bedrock API key (bearer token) MUST NOT be offered in this iteration.
- **FR-004**: The SSO entry MUST accept an AWS CLI profile name and an optional Bedrock region. It MUST NOT ask for, or store, an SSO start URL, SSO region, account, role, access token, refresh token or any other SSO credential; the AWS CLI owns all of that.
- **FR-005**: The IAM connection's region MUST be required and taken from the saved connection, then `AWS_REGION`, then `AWS_DEFAULT_REGION`. The SSO connection's region MUST be taken from its saved region, then `AWS_REGION`, then the profile's configured region as reported by the AWS CLI; when none is available the failure MUST name the setting that supplies it. No region may be guessed.
- **FR-006**: The IAM entry MUST use the saved connection when present and otherwise credentials found in the environment (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN`), following the existing provider precedence and reporting the source in the provider list.
- **FR-007**: The SSO entry MUST resolve credentials only by invoking the AWS CLI for its configured profile and reading the result; it MUST NOT read `~/.aws` token caches, credential files or SSO tokens itself, and MUST NOT start a browser sign-in. An empty profile field MUST mean the CLI's default profile resolution (`AWS_PROFILE`, then `default`).
- **FR-008**: An IAM access key pair with one half missing MUST be refused with a message naming the missing field, and no Bedrock request MUST ever be sent without complete authentication. An SSO connection whose profile cannot be resolved MUST NOT send a request.
- **FR-009**: Every Bedrock request MUST be authentically signed for its target region with either the saved access key pair (including the session token when present) or the temporary credentials the AWS CLI resolves for the SSO profile, so that secrets never travel in a header or query parameter.
- **FR-010**: Bedrock requests MUST go to the regional Bedrock endpoint derived from the Bedrock region of the selected connection, and neither Bedrock entry MUST accept a `base_url` override; an operator who needs another address uses a custom provider.
- **FR-011**: Each connection MUST independently supply chat, embeddings and image generation; changing, expiring or removing one MUST NOT change the other, and both MAY be saved at the same time.
- **FR-012**: Saving, changing or removing a Bedrock connection MUST restart exactly the agents that use that connection, through the existing provider-change path, and MUST NOT affect agents on the other Bedrock connection or on other providers; a later turn succeeds without restarting Vizier.
- **FR-013**: The existing Test action MUST verify the selected connection without starting an agent: the IAM entry checks its credentials, and the SSO entry resolves the profile through the AWS CLI and checks that its role can invoke Bedrock. Both MUST report success or a readable failure and keep the saved connection either way.

**SSO through the AWS CLI**

- **FR-014**: The SSO connection MUST obtain the profile's temporary credentials by invoking the AWS CLI's credential export for that profile (`aws configure export-credentials --profile <profile>` or the documented equivalent), receiving the access key, secret key, optional session token and expiration as structured output.
- **FR-015**: The CLI invocation MUST pass every value as a separate argument with no shell interpolation, MUST be bounded by a timeout, MUST ignore harmless stderr output, and MUST treat a missing executable, unsupported CLI version, non-zero exit, timeout or malformed output as distinct actionable failures.
- **FR-016**: Resolved SSO credentials MUST be cached in memory only, used until shortly before their expiry, and re-resolved through the CLI before a request that would otherwise use expired credentials. Concurrent resolutions for one connection MUST be coalesced into a single CLI invocation.
- **FR-017**: The SSO entry MUST NOT persist any credential or token across restarts. A restart resolves through the CLI again on the next use; if the CLI session is still valid this needs no human action.
- **FR-018**: When the AWS CLI reports that the SSO session has expired or is invalid, the failure MUST name the profile and the exact `aws sso login --profile <profile>` command to run, MUST mark the connection as needing sign-in, MUST NOT be retried in a loop, and MUST clear on the next successful resolution after the command is run, without restarting Vizier or re-saving the connection.
- **FR-019**: SSO failures MUST distinguish a missing or too-old AWS CLI, a missing or unknown profile, an unsigned or expired CLI session, a role that cannot be assumed, a role lacking `bedrock:InvokeModel`, a timeout and malformed CLI output, so that each can be acted on.
- **FR-020**: Secret access keys, session tokens and any CLI-resolved temporary credentials MUST be encrypted at rest where stored and MUST never be returned by any API, screen, log line, error message or telemetry attribute. The IAM access key ID, and the SSO profile name and region, MAY be shown so the person can identify the connection.

**Chat**

- **FR-021**: Chat MUST use Bedrock's unified conversation interface (Converse), with Vizier's system prompt, full multi-turn history, tool definitions, tool results and assistant text translated to its request shape and its response translated back into Vizier's message parts.
- **FR-022**: Bedrock chat models MUST be listed from the existing model catalogue with their identifiers, readable names, context windows, output limits, capabilities and prices, identically for both entries, and choosing one MUST send that identifier unchanged, including cross-region inference profile prefixes.
- **FR-023**: A chat model identifier that is absent from the picker MUST be accepted when typed or supplied through the API, stored exactly as entered and sent unchanged.
- **FR-024**: Tool calling MUST work end to end: tool definitions offered to the model, every requested call executed once through the normal tool path, results returned to the model, several calls per round supported, and the agent's tool-choice restriction honoured where the model supports it.
- **FR-025**: Image input MUST be sent to chat models whose catalogue capabilities include it, in a form Bedrock accepts; for models that cannot accept images, each image MUST be replaced by the existing text note rather than failing the request.
- **FR-026**: The context window, output limit and capabilities MUST come from the same resolution order as other providers (agent override, then catalogue), and the source of the effective context window MUST appear in agent health.
- **FR-027**: Every answered Bedrock chat completion MUST record its input and output token counts in usage accounting, priced at the catalogue's rates when the model is priced, exactly as other providers' completions are.

**Embeddings**

- **FR-028**: Both Bedrock entries MUST be selectable as an agent's embedding provider through the HTTP API and the WebUI, each reusing its own saved connection.
- **FR-029**: An empty embedding model MUST resolve to a documented default Bedrock embedding model that needs no other setting, and an explicitly typed model id MUST be accepted and stored unchanged.
- **FR-030**: Bedrock's Amazon Titan Text Embeddings models and Cohere Embed models offered by Bedrock MUST be supported; a model id outside the supported families MUST fail with a message naming the model and the supported families rather than sending a malformed request.
- **FR-031**: Text sent to Bedrock embeddings MUST be embedded in batches through the existing embedding interface, and the returned vectors MUST feed the existing memory indexer, so saving memories and `memory_search` behave as they do for every other embedding provider.
- **FR-032**: The vector size MUST be the model's own published default (no new dimension setting); a model whose vector size differs from the agent's existing memory index MUST produce an actionable incompatibility error and MUST NOT write vectors of the wrong size.
- **FR-033**: An unusable Bedrock embedding configuration (including an SSO profile that cannot be resolved) MUST fail the agent's build with an actionable error naming Bedrock and the setting or CLI command to fix; an embedding failure during a memory save MUST keep the existing behaviour (the save succeeds and the failure is logged).
- **FR-034**: Agents using local embeddings or another provider's embeddings MUST be unaffected, with no reindex and no configuration change.

**Image generation**

- **FR-035**: Both Bedrock entries MUST be selectable as an agent's image-generation provider through the HTTP API and the WebUI, each reusing its own saved connection.
- **FR-036**: An empty image model MUST resolve to a documented default Bedrock image model that needs no other setting, and an explicitly typed model id MUST be accepted and stored unchanged.
- **FR-037**: Bedrock's Amazon Nova Canvas and Amazon Titan Image Generator models MUST be supported; a model id outside the supported families MUST fail with a message naming the model and the supported families.
- **FR-038**: Image generation MUST accept the prompt and the agent's optional size, use the documented default size when none is given, and send the size in the form the chosen model accepts; an unsupported size MUST surface the provider's explanation.
- **FR-039**: A generated image MUST be saved in the conversation's session files with the media type the provider actually returned and a matching extension, delivered as a channel attachment, and follow the same retention and deletion rules as other generated media.
- **FR-040**: A blocked prompt, an empty result or a provider failure MUST fail the tool call with the provider's explanation, save no partial file, and leave the agent able to continue the turn and later turns.
- **FR-041**: Image and embedding calls MUST NOT be added to chat token accounting, matching the existing treatment of media and embedding calls.

**Cross-cutting**

- **FR-042**: Bedrock's failures MUST be surfaced as actionable errors that preserve the provider's error code and message, covering both connections: missing or incomplete IAM credentials, unsupported Bedrock API key, missing region, missing or too-old AWS CLI, unknown profile, expired CLI session, role without permission, model not available in the region, model access not enabled, expired IAM session token, throttling, validation errors, unsupported model family, service unavailability and timeouts MUST each read differently enough to act on.
- **FR-043**: A failed Bedrock request MUST NOT crash the agent, lose results already produced in the turn, or leave a half-written assistant message; the agent MUST stay available and its health MUST report the failure until it recovers.
- **FR-044**: Retries MUST follow the existing provider policy: throttling and retryable server failures are retried with backoff (honouring the provider's retry hint where present), and an exhausted retry budget ends the turn with the provider's error. An expired CLI session or a missing CLI MUST NOT be retried in a loop.
- **FR-045**: Bedrock MUST work everywhere any other provider works without special cases: ordinary chat on every channel, the dream cycle, scheduled tasks, subagents, background jobs, inter-agent messages, handovers and checkpoints, conversation history that predates the provider switch, and memory indexing for every agent.
- **FR-046**: Startup, provider and agent builds, the scheduler and dreams MUST NOT invoke the AWS CLI or make any AWS request; invocations happen only for a person-triggered test, an agent turn, a memory indexing operation or an image-generation call, and the catalogue's refresh rules are unchanged.
- **FR-047**: Existing providers, agents and stored configurations MUST behave exactly as before; no migration is required, and an agent on another provider MUST NOT be restarted or changed by this feature.
- **FR-048**: The documentation MUST cover both connections, their fields and environment variables (IAM only for the environment), that Bedrock API keys and other credential sources are unsupported, the AWS CLI prerequisite and minimum version for the SSO entry, the `aws sso login` workflow and its manual re-login behaviour, the required AWS permissions, the model-access prerequisite, the regional and cross-region model-id behaviour, the SSO profile's region and the Bedrock region, the supported embedding and image model families with their documented defaults and sizes, the single-region-per-connection and single-profile-per-SSO-connection limitations, and the fact that Bedrock does not provide text-to-speech or speech-to-text; the encryption documentation MUST list the new protected fields.
- **FR-049**: Bedrock's text-to-speech and speech-to-text provider choices MUST NOT be added, because Bedrock exposes no batch API for them; the documentation MUST say so and name the AWS services that would provide them instead.

### Key Entities

- **Bedrock IAM connection**: the saved credential entry with an access key ID, secret access key, optional session token and region, plus where the values came from (saved or an environment variable). Secrets are write-only and encrypted; the key ID and region are identifiers.
- **Bedrock SSO connection**: the saved entry with an AWS CLI profile name and an optional Bedrock region, plus the in-memory temporary credentials the CLI resolves for that profile and their expiry. No SSO token or credential is persisted; the profile name and region are not secrets.
- **Bedrock chat model entry**: a catalogue model of the Bedrock provider with its identifier (possibly a cross-region inference profile), readable name, context window, output limit, capabilities (tool calling, image input) and price; shared by both connections and the source of the picker.
- **Bedrock embedding selection**: the agent's chosen Bedrock connection and optional model id, resolved to a supported Bedrock embedding family, its vector size and that connection's credentials; the memory indexer's embedding source.
- **Bedrock image selection**: the agent's chosen Bedrock connection, optional model id and optional size, resolved to a supported Bedrock image family and that connection's credentials.
- **Agent provider selection**: an agent's provider and model for chat, plus its embedding and image settings, each resolved at build time into the effective endpoint, credential and limits.
- **Bedrock conversation request/response**: the translated chat turn — system prompt, messages, tool definitions and results, images — and the model's answer parts, stop reason and token usage.
- **Generated media artifact**: an image file attached to a conversation's session files with the provider's media type, produced by Bedrock or any other image provider.
- **Usage record**: one row per answered chat completion with provider, model, input and output tokens, activity and session, priced later from the catalogue.
- **Provider health**: the state an affected agent reports when its Bedrock request fails, including the last error and, when healthy, the effective context window and its source.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A person with an AWS account, a region and IAM keys can connect Bedrock and get a first chat reply in under 3 minutes, with no step outside Settings › Providers and the agent editor.
- **SC-002**: A person who has already run `aws sso login --profile <profile>` can save the SSO connection and get a first chat reply in under 3 minutes, entering only the profile name and (optionally) a region.
- **SC-003**: 100% of the verification chat turns in the matrix — first turn, multi-turn, system prompt and CORE, tool call, several tool calls, tool result, image to a vision model, image to a text-only model, cross-region model id, checkpointed long conversation — complete with the expected reply or behaviour, through either Bedrock connection.
- **SC-004**: In 100% of missing-credential, incomplete-pair, missing-region, wrong-region, missing-CLI, too-old-CLI, unknown-profile and unsupported-API-key trials, the person sees an actionable message naming the setting, profile, command or environment variable to fix, and no unauthenticated request is sent.
- **SC-005**: In 100% of checks, no secret access key, session token, CLI-resolved credential, SSO refresh token or SSO access token appears in an API response, WebUI screen, log line, error message or telemetry attribute; the IAM access key ID, and the SSO profile name and region, remain visible.
- **SC-006**: 100% of tool-using turns execute each requested call exactly once, return every result to the model, and end with the model's final answer rather than a provider error.
- **SC-007**: A vision-capable Bedrock chat model answers correctly about an image in 100% of runs, and a text-only Bedrock chat model answers with the image replaced by a note in 100% of runs.
- **SC-008**: With either Bedrock connection selected for embeddings and the model left empty, 100% of newly saved memories are embedded and `memory_search` returns the saved passage for a distinctive phrase in 100% of runs; the same holds for an explicitly chosen Titan or Cohere model from the supported families.
- **SC-009**: Selecting a supported Bedrock embedding model whose vector size differs from the agent's existing index produces a clear incompatibility error in 100% of trials and writes no wrong-sized vectors; agents on local or other embeddings are unchanged.
- **SC-010**: With either Bedrock connection selected for image generation and the model left empty, 100% of prompts produce a valid image in the conversation's files that opens in a standard viewer; an explicit supported model and size are honoured or the provider's refusal is explained; unsupported model ids and blocked prompts produce actionable errors with no broken file.
- **SC-011**: 100% of answered Bedrock chat completions appear in the usage views with non-zero token counts where the provider reports them, and their estimated cost matches the catalogue's rate for the model; models with no catalogue price show no invented cost, and embedding and image calls are not counted as chat tokens.
- **SC-012**: After `aws sso login --profile <profile>` is run, 100% of expired-session trials recover on the next turn with no connection edit, no Vizier restart and no repeated retry loop; the failure before recovery names the exact command and profile.
- **SC-013**: After invalid IAM credentials are replaced with valid ones, the next chat turn succeeds with no Vizier restart; agents on the other Bedrock connection and on other providers are never restarted by a change, expiry or removal of one connection.
- **SC-014**: A Vizier restart while the CLI session is valid leaves the SSO connection working in 100% of trials, and no SSO token or CLI-resolved credential is persisted by Vizier at any point.
- **SC-015**: 100% of the existing provider and agent regression checks still pass; no existing stored configuration changes behaviour and no migration runs.
- **SC-016**: Zero AWS CLI invocations and zero AWS requests are made at startup, during agent builds, by the scheduler or by the dream cycle; every invocation and request is attributable to a person-triggered test, an agent turn, a memory indexing operation or an image-generation call.
- **SC-017**: A person finds both Bedrock entries in Settings › Providers and a Bedrock chat model in an agent's picker on the first attempt, and sets Bedrock embeddings or image generation without leaving the agent editor, in 100% of usability trials.
- **SC-018**: In 100% of injected provider failures (throttling, model not enabled, permission denied, expired IAM token, expired CLI session, missing CLI, unknown profile, invalid model id, unsupported model family, oversized request, blocked prompt), the failure is reported with the provider's explanation, the agent remains able to run a later turn, and no partial assistant message or broken artifact is stored.

## Assumptions

- Two provider identifiers are used: the catalogue's `amazon-bedrock` for **Amazon Bedrock (IAM)**, and a built-in `amazon-bedrock-sso` for **Amazon Bedrock (SSO)** that borrows the same catalogue model list. Both display names carry the parenthesised authentication method so they are distinguishable in every picker.
- Chat models, limits, capabilities and prices continue to come from the existing catalogue (models.dev); the SSO entry does not need its own catalogue entry to list them.
- "SSO" means an AWS CLI profile that is already signed in through AWS IAM Identity Center. Vizier resolves that profile's credentials by invoking the AWS CLI; it does not implement the SSO sign-in flow, does not open a browser, does not read `~/.aws` caches or token files directly, and does not store SSO tokens or refresh tokens.
- The AWS CLI must be installed, executable and on the process's `PATH` for the SSO entry, at a version that supports exporting credentials for a profile (`aws configure export-credentials`, AWS CLI v2.13 or later). This is an opt-in dependency of the SSO entry only; the IAM entry has no CLI dependency, and a missing or too-old CLI degrades to an actionable error.
- The person is responsible for running `aws sso login --profile <profile>` on the machine running Vizier, and again whenever the CLI session expires (AWS typically keeps an SSO session for hours; the CLI decides). Vizier surfaces the exact command when the session is no longer valid and does not attempt to sign in itself.
- Vizier does not install or bundle the AWS CLI. The SSO entry requires it on the Vizier process's executable path and uses the normal AWS configuration (`~/.aws`, `AWS_CONFIG_FILE`, `AWS_SHARED_CREDENTIALS_FILE`), which the CLI already honours. Where the CLI is unavailable, the SSO entry fails with the actionable CLI-missing error and the IAM entry is unaffected.
- An SSO connection holds one profile (or the CLI's default profile when the field is empty) and one Bedrock region. Multiple profiles in one connection, switching profiles per request, browser sign-in, account/role browsing and in-app device authorization are out of scope.
- The CLI resolves the profile's credential source, which is normally an SSO role but may be static keys, an assumed role or a credential process; those work, but the SSO profile is the documented and supported use.
- The SSO region is the CLI's business; Vizier only needs the Bedrock region. When the saved override is empty, `AWS_REGION` and then the profile's configured region (as reported by the CLI) are used, and a missing region is an actionable error.
- Only IAM access keys and CLI-resolved profile credentials are supported for authentication. Bedrock API keys (bearer tokens), reading the CLI cache directly, and storing SSO sessions are out of scope for this iteration; `AWS_BEARER_TOKEN_BEDROCK` is recognised only to explain that it is unsupported, not to authenticate.
- Supported embedding families are Amazon Titan Text Embeddings V2 (the documented default, 1024 dimensions) and Titan Text Embeddings V1, plus Cohere Embed English and Multilingual v3 as offered by Bedrock. They all default to a 1024-dimension vector, so they remain compatible with each other in one index. A dimension setting, Nova multimodal embeddings, asynchronous embedding jobs and custom output dimensions are out of scope.
- Supported image-generation families are Amazon Nova Canvas (the documented default, 1024×1024 when no size is set) and Amazon Titan Image Generator V2. Stability AI models on Bedrock, image-to-image, negative prompts, seeds, quality controls, multiple images per request and video generation (Nova Reel) are out of scope.
- Bedrock exposes no batch text-to-speech or speech-to-text API (its speech model is real-time bidirectional), so TTS and STT keep their current providers; Amazon Polly and Amazon Transcribe are separate services and are not wired in here.
- Model availability and model access are the operator's responsibility on AWS: a chat model may be listed in the catalogue but not enabled for the account or offered in the chosen region, and the provider's error is authoritative. Vizier does not silently substitute another model. Embedding and image model ids are not in the chat catalogue, so they are chosen from the documented supported families or typed.
- Streaming responses are not used: chat turns receive the complete response, as with every provider except the one that requires streaming. The `InvokeModelWithResponseStream` action is not required.
- Extended thinking, reasoning-content display, prompt caching, structured outputs, Guardrails, provisioned throughput, Batch inference, Bedrock Agents and Bedrock Knowledge Bases are out of scope. Chat responses containing blocks Vizier does not map must still produce a usable turn.
- Both connections are opt-in: a fresh install makes no AWS call and needs no AWS configuration, preserving config-less startup. Only a person-triggered action, an agent turn, a memory indexing operation or an image-generation call reaches AWS or the CLI.
- The required AWS permission is `bedrock:InvokeModel` for chat, embeddings and image generation (the connection test uses the same invocation path); the exact policy is documented in the same change, together with the IAM Identity Center permission set requirements.
- Prices come from the catalogue and may be missing for a chat model or become stale; usage without a price is still recorded and shown without an invented cost estimate. Embedding and image calls stay outside chat token accounting, as existing media calls do.
- The existing retry policy, timeouts and error classification are reused rather than reinvented for Bedrock; only Bedrock-specific and CLI/SSO-specific error translation is added.
- The WebUI connect form is generic over declared fields, so the new credential fields appear without redesigning Settings › Providers. The existing embedding and image model fields remain free text, and the existing chat model picker lists catalogue models once the entries become selectable.
- Verification uses a live AWS account and, for SSO, a live CLI profile signed in to an IAM Identity Center instance, because no offline server speaks Bedrock; those steps are marked as live-provider steps in the quickstart, and the dummyplug-based checks cover the parts that do not need AWS.
