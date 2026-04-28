# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 2.0.x  | :white_check_mark: |
| 1.x    | :x:                |


## Security Model
Justice AI is designed with privacy as a core principle. All document processing, embedding, and LLM inference run entirely on your local machine. No user data, documents, or queries are transmitted to external servers. The only network requests made by the application are:

Initial model download from HuggingFace (huggingface.co) on first launch
No telemetry, analytics, or crash reporting of any kind

## Reporting a Vulnerability
If you discover a security vulnerability in Justice AI, please do not open a public GitHub issue. Instead, report it privately through one of the following channels:

GitHub Private Vulnerability Reporting — use the "Report a vulnerability" button under the Security tab of this repository
Email — contact the maintainers directly via the email listed on the GitHub profile

Please include in your report:

A description of the vulnerability and its potential impact
Steps to reproduce the issue
Any relevant logs, screenshots, or proof-of-concept code

You can expect an acknowledgment within 72 hours and a resolution timeline within 14 days depending on severity.

## Scope
The following are considered in scope for security reports:

Local data handling and storage (chat history, document chunks, settings)
The encrypted vault component (security/vault/)
Network requests made by the application
WebView2 content security policy bypasses
Tauri IPC command injection or privilege escalation

The following are out of scope:

Vulnerabilities in third-party models (Qwen3-8B) or the HuggingFace platform
Social engineering attacks
Physical access attacks

## Known Limitations

Chat history and document chunks are stored locally and encrypted at rest. However, the encryption key is derived from device-local state and does not require a user password in the current version.
The application does not verify the integrity of downloaded model files beyond checking the GGUF header magic bytes.
