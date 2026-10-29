# Tasks: Amazon Bedrock AWS SSO Provider

- [X] Add separate provider identity, generic `aws_profile` storage/API field, and validation.
- [X] Add AWS CLI credential-process broker with in-memory expiring cache and SigV4 integration.
- [X] Reuse Bedrock catalogue, dialect, custom model IDs, embeddings and images through SSO provider variants.
- [X] Add setup guide and provider documentation.
- [ ] Run `cargo test`, `cargo clippy`, and live AWS SSO verification with a Rust >=1.96 toolchain.
