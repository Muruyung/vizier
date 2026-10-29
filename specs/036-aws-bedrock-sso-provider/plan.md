# Implementation Plan: Amazon Bedrock AWS SSO Provider

`amazon-bedrock-sso` is a virtual provider over the existing Bedrock catalogue and `DialectKind::Bedrock`. Its `AwsCliSsoCredential` delegates profile/cache/OIDC handling to AWS CLI v2 via `aws configure export-credentials --format process`; temporary output is cached in memory only and signed through the existing SigV4 primitive. Static `amazon-bedrock` remains access-key-only. The model field remains a free string so custom model IDs, inference-profile IDs and ARNs are preserved.

Validation: Rust resolver/schema/guide tests, formatter and clippy/test suite when a Rust >=1.96 toolchain is available; live verification runs `aws sso login`, provider Test, a Bedrock chat, and a custom model ID.