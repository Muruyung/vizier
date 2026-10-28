# Tasks: AWS Bedrock Provider

**Input**: Design documents from `/specs/034-aws-bedrock-provider/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/, quickstart.md

**Tests**: Offline tests are included because the plan's verification strategy and the constitution require them (`cargo test`, `cargo clippy`, WebUI tests/typecheck). **Live Bedrock is not run by the implementer** — the maintainers have no Bedrock account. `quickstart.md` §5–§6 are packaged for a volunteer verifier on the special release (T046); they are not CI tasks.

**Organization**: Tasks are grouped by user story so each slice can be implemented and verified independently. US1 and US2 are both P1; US2 extends the Converse dialect US1 introduces. US3/US4 share `src/llm/media/bedrock.rs` as a serialization point. US5 is the operational slice over all of them.

**Before you start**: read [contracts/bedrock-wire.md](./contracts/bedrock-wire.md), [contracts/provider-credentials-api.md](./contracts/provider-credentials-api.md), [contracts/provider-layer.md](./contracts/provider-layer.md) and [data-model.md](./data-model.md) for the exact wire shapes, field rules, defaults and error mapping. Do not improvise the SigV4 canonicalisation, the Converse block shapes or the provider field names.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1–US5)
- Include exact file paths in descriptions

## Path Conventions

Single Rust service with the existing layout: `src/`, `webui/app/`, `docs/src/`, `specs/034-aws-bedrock-provider/`. No new crate, no new package.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Provider identity, credential schema and the data-driven listing every story reads.

- [ ] T001 [P] Add `ProviderId::AMAZON_BEDROCK` in `src/config/provider.rs` and `DialectKind::Bedrock` (serde/utoipa `snake_case`) in `src/llm/dialect/mod.rs`.
- [ ] T002 [P] Map `@ai-sdk/amazon-bedrock` to `DialectKind::Bedrock` in `src/catalogue/data.rs`; replace the `bedrock` "unsupported" fixture in `src/catalogue/data.rs` and `src/catalogue/mod.rs` tests with a provider that has no dialect, and assert Bedrock is listed.
- [ ] T003 [P] Add `AccessKeyId`, `SecretAccessKey`, `SessionToken`, `Region` to `CredentialField` in `src/catalogue/builtin.rs`, and add `access_key_id`, `secret_access_key`, `session_token`, `region` to `ProviderCredentials` in `src/schema/provider.rs` (including `is_empty` and `merged_with`; fields are `Option<String>`).
- [ ] T004 [P] Add WebUI field labels and plain-field membership for the four fields in `webui/app/lib/providers.ts` and the corresponding types in `webui/app/api/endpoints/providers.ts` (`Access key ID`, `Secret access key`, `Session token (optional)`, `Region`; `access_key_id` and `region` are plain).
- [ ] T005 Add the Bedrock listing rule in `src/catalogue/mod.rs`: fields exactly `[AccessKeyId, SecretAccessKey, SessionToken, Region]` (no `api_key`), and a shared `extra_env` that appends `AWS_SESSION_TOKEN` and `AWS_DEFAULT_REGION` to the catalogue's AWS list; add `amazon-bedrock` to `MEDIA_PROVIDERS` in `src/channels/http/api/v1/catalogue.rs`.

**Checkpoint**: The provider is visible, the four fields exist end to end, and the listing is authoritative.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Signing, credentials, resolver, media transport and the chat dialect core. No user story can work without these.

**⚠️ CRITICAL**: Complete this phase before US1–US5 implementation.

- [ ] T006 [P] Implement the pure SigV4 signer in `src/llm/sigv4.rs`: canonical request, `x-amz-date`, signed headers (`content-type`, `host`, `x-amz-date`, `x-amz-security-token`), double-encoded canonical path, signing-key chain, `Authorization` header. Encode model ids per the Smithy `BASE_SET` (unreserved `A-Za-z0-9-._~` pass; `:` and `/` become `%XX`, uppercase) exactly once, and the canonical URI a second time (`%` → `%25`). Add tests against the AWS SigV4 vectors in the same file.
- [ ] T007 Add `SigV4Credential` and the default `authorize_with_body(req, info, method, url, body)` method in `src/llm/auth.rs`; serialize the body once and send the signed bytes in `src/llm/model.rs`; add AWS `Authorization` and `x-amz-security-token` redaction in `src/llm/http.rs`.
- [ ] T008 [P] Implement `src/llm/bedrock.rs`: `aws_uri_encode` (shared with T006), region/endpoint helpers, and `ListFoundationModels` (`GET https://bedrock.{region}.amazonaws.com/foundation-models`, count `modelSummaries`), with tests for encoding, endpoint and the count.
- [ ] T009 [P] Extend `src/provider_keys.rs` with explicit mappings (`AWS_ACCESS_KEY_ID` → access key ID, `AWS_SECRET_ACCESS_KEY` → secret, `AWS_SESSION_TOKEN` → session token, `AWS_REGION`/`AWS_DEFAULT_REGION` → region, `AWS_BEARER_TOKEN_BEDROCK` → api_key only for the unsupported message) before the `contains("KEY")` heuristics; share `extra_env` with T005; add tests.
- [ ] T010 [P] Protect and decrypt `credentials.secret_access_key` and `credentials.session_token` in `src/encryption/fields.rs` with the existing record-bound AAD; keep `access_key_id`/`region` readable; add round-trip and sibling-unchanged tests.
- [ ] T011 Extend `src/llm/resolve.rs`: Bedrock region precedence (saved → `AWS_REGION` → `AWS_DEFAULT_REGION`), region format `^[a-z]{2,4}(-[a-z0-9]+)+-\d+$`, incomplete-pair and bearer-only errors, `https://bedrock-runtime.{region}.amazonaws.com`, `SigV4Credential`, no `base_url`; add tests (including the old "bedrock unsupported" expectation).
- [ ] T012 Add `Endpoint::post_json` in `src/llm/media/openai_shaped.rs` (serialize once, `authorize_with_body`, send the same bytes, 300 s timeout) and the Bedrock branch in `src/llm/media/mod.rs` (same region/pair rules, runtime base URL, `SigV4Credential`).
- [ ] T013 Implement the chat core of `src/llm/dialect/bedrock.rs`: `SystemStyle::Field`, `/model/{encoded}/converse`, system blocks, alternating messages (merge consecutive same-role), text, `inferenceConfig.maxTokens` (agent → catalogue output), `usage`, stop reasons and empty `end_turn`; tests for multi-turn, usage and unknown stop reasons.
- [ ] T014 Extend `src/channels/http/api/v1/providers/mod.rs` with the four fields in `UpsertProviderRequest`/`ProviderResponse`, `sent()`/`apply`/`field_name`, `access_key_id`/`region` in the response, `has_secret_access_key`/`has_session_token` booleans, and the "exactly one half" refusal; add save/mask/refusal tests.
- [ ] T015 Regenerate the WebUI API types after T014 with `node scripts/openapi.mjs` (`webui/app/api/schema.d.ts`) and run `cd webui && npm run typecheck`.

**Checkpoint**: Signing, credentials, resolver, media transport and chat translation are ready; US1 can start.

---

## Phase 3: User Story 1 - Connect AWS Bedrock and talk to a model (Priority: P1) 🎯 MVP

**Goal**: Amazon Bedrock is connectable with IAM keys, the form teaches the setup, Test works without model access, and an agent can hold a multi-turn chat.

**Independent Test**: Offline: `GET /api/v1/catalogue/providers` lists Bedrock with four fields and the guide; `PUT` masks the secrets; the resolver returns the expected region/pair errors; dialect fixtures decode a Converse reply. Live: the volunteer follows `quickstart.md` §5.2–§5.4.

### Implementation for User Story 1

- [ ] T016 [P] [US1] Add `src/catalogue/guide.rs` with the static Bedrock guide (five steps, the exact policy `["bedrock:InvokeModel","bedrock:ListFoundationModels"]`, AWS links and `https://www.vizier.rs/docs/configuration/providers#amazon-bedrock`) and expose it as `ProviderSummary.guide` in `src/channels/http/api/v1/catalogue.rs`; add a listing test asserting the fields and guide.
- [ ] T017 [US1] Render the guide in `webui/app/screens/providers/ConnectForm.tsx`: steps, copyable policy, links, open for a new connection and reachable while editing/after a failed Test; add the `ProviderGuide` type in `webui/app/api/endpoints/catalogue.ts`; add a component test.
- [ ] T018 [P] [US1] Add the Bedrock Test branch in `src/channels/http/api/v1/providers/usage.rs`: `ListFoundationModels` with the SigV4 credential, 10–15 s timeout, one attempt, `model_count` on success, `TestResult::failed` on error; test the helper's count/error shaping.
- [ ] T019 [P] [US1] Add Converse chat fixtures in `src/llm/dialect/bedrock.rs`: system + multi-turn text, empty `end_turn`, usage totals, `tool_use`/`max_tokens`/`guardrail_intervened` stop reasons, and error-in-body handling.
- [ ] T020 [P] [US1] Add the **Amazon Bedrock** section to `docs/src/configuration/providers.md` (five setup steps, policy, IAM-only auth, `AWS_*` variables, region, cross-region ids, single-region limitation, TTS/STT absence) with the `#amazon-bedrock` anchor; link it from the guide.
- [ ] T021 [US1] Wire and verify the offline connect/chat slice: provider listing + guide tests in `src/channels/http/api/v1/catalogue.rs`, save/mask tests in `src/channels/http/api/v1/providers/mod.rs`, and the quickstart §2 form/API walkthrough; record any failure in the task notes.

**Checkpoint**: US1 is independently demonstrable offline; the live chat check is the volunteer's §5.

---

## Phase 4: User Story 2 - Tools and image input (Priority: P1)

**Goal**: Tool calls, multiple tool results, tool choice and vision work through the same Converse dialect; a text-only model gets the note.

**Independent Test**: Dialect fixtures: `toolUse` decodes to `AssistantPart::ToolCall`, consecutive `toolResult`s merge into one user message, an image encodes as an `image` block, and a text-only model's image is replaced by the shared note. Live: `quickstart.md` §5.5–§5.6.

### Implementation for User Story 2

- [ ] T022 [US2] Extend `src/llm/dialect/bedrock.rs` with `toolUse`/`toolResult` blocks, consecutive-result grouping, `toolConfig`/`toolChoice` (auto; `None` omits the choice and keeps tools), image blocks (`png`/`jpeg`/`gif`/`webp`), unsupported-image notes, and reasoning/unknown-block handling; add tests for several calls/results and images.
- [ ] T023 [P] [US2] Add `prepare` capability tests for Bedrock in `src/llm/prepare.rs`: an image is kept when capabilities include it and replaced by the note when `image_input() == Some(false)`; a non-image attachment is replaced.
- [ ] T024 [US2] Add a tool-loop fixture test in `src/llm/dialect/bedrock.rs` (or `src/llm/dialect/golden_tests.rs`): decode a `toolUse`, feed the tool result back, encode the next request, and assert no call is duplicated and the result is a user `toolResult` block.

**Checkpoint**: US1 + US2 both work; the live tool/vision checks are the volunteer's §5.5–§5.6.

---

## Phase 5: User Story 3 - Memory embeddings through Bedrock (Priority: P2)

**Goal**: Bedrock is selectable for embeddings; Titan V2 is the default, Titan V1 and Cohere v3 work; memory save/search is unchanged; a dimension mismatch fails clearly.

**Independent Test**: `src/llm/media/bedrock.rs` tests pin Titan/Cohere bodies and batching; `src/indexer/sqlite.rs` test pins the dimension error; offline `memory_search` cannot call AWS, so the live recall check is the volunteer's §6.1–§6.3.

### Implementation for User Story 3

- [ ] T025 [P] [US3] Add `EmbeddingProvider::Bedrock` in `src/schema/agent.rs` with `credentials() == Some(ProviderId::AMAZON_BEDROCK)`, `display_name`, and the default model `amazon.titan-embed-text-v2:0`; add schema tests.
- [ ] T026 [US3] Implement `BedrockEmbeddings` in `src/llm/media/bedrock.rs`: Titan V2/V1 (`inputText`, `normalize: true`) and Cohere v3 (`texts`, `input_type: "search_document"`, chunks of 96); family detection strips an optional `us.`/`eu.`/`apac.`/`global.` prefix; unknown ids name the supported families; tests for request bodies, response parsing and batch length.
- [ ] T027 [US3] Wire `src/embedding/mod.rs`: empty model → Titan V2, build `BedrockEmbeddings` via `media::endpoint`, and surface the family error at build time.
- [ ] T028 [US3] Add the index dimension guard in `src/indexer/sqlite.rs`: read the existing `document_index` `float[N]` DDL, compare with the probe vector, and return a clear error naming both sizes and the reindex requirement; test with a mismatched declared size.
- [ ] T029 [P] [US3] Add `bedrock` to `EMBEDDING_PROVIDERS` and the model hint/default in `webui/app/lib/agentForm.ts`; add the corresponding form test.
- [ ] T030 [P] [US3] Add the Bedrock embedding families/defaults to `docs/src/configuration/tools-embedding.md` and cross-link the provider section.

**Checkpoint**: Embedding selection, wire clients and the index guard are complete; live recall is the volunteer's §6.1–§6.3.

---

## Phase 6: User Story 4 - Image generation through Bedrock (Priority: P2)

**Goal**: Bedrock is selectable for image generation; Nova Canvas is the default, Titan Image Generator V2 works; size, MIME and RAI errors behave.

**Independent Test**: `src/llm/media/bedrock.rs` tests pin the Nova/Titan request bodies, size parsing, base64 decode, RAI `error` and MIME sniff; the live image check is the volunteer's §6.4–§6.5.

### Implementation for User Story 4

- [ ] T031 [P] [US4] Add `ImageGenProvider::Bedrock` in `src/schema/agent.rs` with `credentials() == Some(ProviderId::AMAZON_BEDROCK)` and the default `amazon.nova-canvas-v1:0`; add schema tests.
- [ ] T032 [US4] Implement `BedrockImages` in `src/llm/media/bedrock.rs`: `TEXT_IMAGE`, `numberOfImages: 1`, `quality: "standard"`, `cfgScale: 8.0`, default 1024×1024, strict `WxH` parse, Nova/Titan family detection, first-image decode, RAI `error` as a tool error, PNG/JPEG/WEBP MIME sniff; tests for every branch.
- [ ] T033 [US4] Wire `src/image_generation/mod.rs` to build `BedrockImages` with the saved connection and region.
- [ ] T034 [P] [US4] Add `bedrock` to `IMAGE_GEN_PROVIDERS` and the default model in `webui/app/lib/agentForm.ts`; add the form test.
- [ ] T035 [P] [US4] Add the Bedrock image families/default/size rules to `docs/src/configuration/tools-embedding.md`.

**Checkpoint**: Image generation is complete offline; live generation is the volunteer's §6.4–§6.5.

---

## Phase 7: User Story 5 - Credentials, health and cost (Priority: P2)

**Goal**: Rotation, removal, region changes, usage/cost and agent health behave like every other provider; secrets stay masked; one change affects only Bedrock agents.

**Independent Test**: API/encryption tests cover masking and rotation; a usage fixture prices a Converse reply from the catalogue; the restart path is the existing provider-change path. Live rotation/error checks are the volunteer's §5/§6.7.

### Implementation for User Story 5

- [ ] T036 [US5] Extend `src/channels/http/api/v1/providers/usage.rs` `features`/`used_by_all` so a Bedrock-configured chat, embedding and image agent counts under `amazon-bedrock`; add a used-by test and verify the existing provider-change restart path is the only restart trigger.
- [ ] T037 [P] [US5] Add a usage/cost fixture test for a Converse reply in `src/usage/` (input/output tokens, catalogue price, no cost when unpriced) and confirm embeddings/images stay out of chat accounting.
- [ ] T038 [P] [US5] Update `webui/app/lib/providers.ts` `credentialText` for Bedrock (`Saved keys` / `Keys from AWS_ACCESS_KEY_ID`, region shown) and add a test.
- [ ] T039 [P] [US5] Add the two protected fields to `docs/src/configuration/encryption.md`; add the credential lifecycle/rotation/removal and error explanations to the Amazon Bedrock provider section.
- [ ] T040 [US5] Add API tests in `src/channels/http/api/v1/providers/mod.rs` for a complete save, masked response, invalid region, incomplete pair, bearer-only rejection and deletion; add encryption round-trip coverage for the new fields in `src/encryption/fields.rs` if T010 did not already.

**Checkpoint**: Operational behaviour is covered offline; live rotation/error checks are the volunteer's §5/§6.7.

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Schema/docs consistency, the offline gate and the volunteer release package.

- [ ] T041 [P] Regenerate `webui/app/api/schema.d.ts` with `node scripts/openapi.mjs` after the API/guide types settle, and run `cd webui && npm run typecheck`.
- [ ] T042 [P] Check the three docs pages agree (`docs/src/configuration/providers.md`, `tools-embedding.md`, `encryption.md`): the guide steps, policy, defaults, protected fields, anchor and TTS/STT absence; fix drift.
- [ ] T043 Run the full offline gate (`Cargo.toml`, `webui/package.json`) from the repository root: `cargo test`, `cargo clippy`, `cd webui && npm test && npm run typecheck`; fix warnings/errors.
- [ ] T044 Run `specs/034-aws-bedrock-provider/quickstart.md` §1–§4 on a fresh workspace (`--data-dir /tmp/vizier-bedrock-quickstart`), including the dummyplug regression and the no-startup-AWS-traffic check; record results in the task notes.
- [ ] T045 [P] Verify no new dependency and no migration/legacy entry: inspect `Cargo.toml`/`Cargo.lock`, confirm `ring`/`chrono` only, and confirm no `#82` item is needed.
- [ ] T046 Package the volunteer verification: finalize `specs/034-aws-bedrock-provider/quickstart.md` §5–§6 and the Verifier report, attach it to the special-release notes/tracking issue, and state that live failures are fixed before stable. Do not run the live steps.
- [ ] T047 [P] Add the Bedrock provider/dialect/credential notes to `CLAUDE.md`'s Providers section and check the docs link `https://www.vizier.rs/docs/configuration/providers#amazon-bedrock` resolves to the new section.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies.
- **Foundational (Phase 2)**: depends on Setup; blocks all user stories.
- **US1 (Phase 3)**: depends on Foundational (T006–T015).
- **US2 (Phase 4)**: depends on US1's dialect (T013) and extends it; can follow US1 immediately.
- **US3 (Phase 5)**: depends on Foundational (T008, T012, T014) and can run in parallel with US2/US4, but shares `src/llm/media/bedrock.rs` with US4.
- **US4 (Phase 6)**: depends on Foundational (T008, T012, T014); shares `src/llm/media/bedrock.rs` with US3.
- **US5 (Phase 7)**: depends on US1 (T018 Test/API) and the US3/US4 selections being present; can run after US1 and alongside late US3/US4 work.
- **Polish (Phase 8)**: depends on all selected stories.

### User Story Dependencies

- **US1 (P1)**: independent after Foundational — the MVP.
- **US2 (P1)**: needs US1's dialect; no separate provider or credential work.
- **US3 (P2)**: needs Foundational only; independent of US2/US4 except the shared media file.
- **US4 (P2)**: needs Foundational only; independent of US3 except the shared media file.
- **US5 (P2)**: needs US1's Test/API and the capability selections; integration-only.

### Within Each User Story

- Dialect/dialect core before tool/image extensions.
- Media client before factory wiring.
- Schema variant before WebUI/default/docs.
- Offline tests before the story checkpoint.
- Live checks are packaged (T046), never run by the implementer.

### Parallel Opportunities

- Setup: T001–T004; T005 after T003.
- Foundational: T006/T008/T009/T010 can start together; T007 after T006; T011 after T006–T009; T012 after T007/T008/T009; T013 after T008; T014 after T003/T009; T015 after T014.
- US1: T016 (guide), T018 (Test), T019 (fixtures), T020 (docs) can run in parallel; T017 after T016; T021 after T016–T018.
- US3: T025, T028, T029, T030 can run in parallel; T026 → T027; T026 and T032 are serialized on `src/llm/media/bedrock.rs`.
- US4: T031, T034, T035 can run in parallel; T032 → T033.
- US5: T037, T038, T039 can run in parallel; T036/T040 after US1/US3/US4.

---

## Parallel Example: User Story 1

```bash
# Backend guide, Test, fixtures and docs can start together after Foundational:
Task: "Add src/catalogue/guide.rs and ProviderSummary.guide in src/channels/http/api/v1/catalogue.rs"
Task: "Add the Bedrock Test branch in src/channels/http/api/v1/providers/usage.rs"
Task: "Add Converse chat fixtures in src/llm/dialect/bedrock.rs"
Task: "Add the Amazon Bedrock section to docs/src/configuration/providers.md"
```

## Parallel Example: User Story 3

```bash
Task: "Add EmbeddingProvider::Bedrock in src/schema/agent.rs"
Task: "Add the index dimension guard in src/indexer/sqlite.rs"
Task: "Add bedrock to EMBEDDING_PROVIDERS in webui/app/lib/agentForm.ts"
Task: "Add Bedrock embedding docs to docs/src/configuration/tools-embedding.md"
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Complete Setup + Foundational.
2. Complete US1 (connect, guide, Test, chat fixtures).
3. **STOP and VALIDATE** offline: listing/guide/API mask/resolver errors + dialect fixtures.
4. US2 is the second P1 slice and should ship in the same special release; then US3/US4, then US5.

### Incremental Delivery

1. Setup + Foundational → foundation ready.
2. US1 → offline validation → volunteer can run §5.2–§5.4.
3. US2 → offline fixtures → volunteer can run §5.5–§5.6.
4. US3 → offline fixture/index tests → volunteer can run §6.1–§6.3.
5. US4 → offline fixture tests → volunteer can run §6.4–§6.5.
6. US5 → offline operational tests → volunteer can run §5/§6.7.
7. T046 hands the whole checklist to the volunteer; live failures are fixed before stable.

### Parallel Team Strategy

- One developer: Setup → Foundational → US1 → US2 → US3 → US4 → US5 → Polish.
- Two developers: after Foundational, A takes US1→US2 (dialect), B takes US3→US4 (media); coordinate `src/llm/media/bedrock.rs`; US5 after both.
- US1's guide/Test/docs and US3/US4's WebUI/docs tasks can be split further.

---

## Notes

- `[P]` means different files and no incomplete dependency; the shared `src/llm/media/bedrock.rs` and `src/llm/dialect/bedrock.rs` are serialization points.
- Live AWS checks are not run by the implementer; T046 packages them for the volunteer on the special release.
- Tests are offline only: SigV4 vectors, dialect fixtures, resolver/credentials/encryption, media bodies, index guard, WebUI typecheck.
- Each task should be committed as a logical change; the final gate is T043 + T044.
