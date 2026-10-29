# Feature Specification: Amazon Bedrock AWS SSO Provider

## Goal

Add `amazon-bedrock-sso`, separate from static-key `amazon-bedrock`. It uses an AWS CLI v2 IAM Identity Center profile authenticated beforehand with `aws sso login --profile <profile>`.

## Requirements

- Store only `aws_profile` and `region`; reject static Bedrock fields on the SSO provider.
- Invoke `aws configure export-credentials --profile <profile> --format process` without a shell, browser, or auto prompt.
- Keep returned temporary credentials only in memory, refresh them before expiry, sign every request with existing Bedrock SigV4, and never log secret output.
- Reuse Converse chat, Bedrock embedding/image clients, regional endpoints, model catalogue, and typed custom model IDs.
- A missing CLI/login/profile must tell the operator to run `aws sso login --profile <profile>` on the Vizier host.
- No AWS or CLI request occurs at startup or agent build; the first signed call and Test may acquire credentials.
