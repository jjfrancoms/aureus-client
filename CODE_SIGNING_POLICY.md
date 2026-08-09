# Code signing policy

Free code signing provided by [SignPath.io](https://signpath.io/), certificate by
[SignPath Foundation](https://signpath.org/).

## Scope

Only release artifacts built from a public `v*` tag by the repository's GitHub
Actions workflow are eligible for signing. Windows signing will become active
after SignPath approves Aureus and the CI integration is configured. Tauri
updater artifacts are independently signed and verified before installation.

## Roles

- Committer and reviewer: [jjfrancoms](https://github.com/jjfrancoms)
- Approver: [jjfrancoms](https://github.com/jjfrancoms)

Changes proposed by other contributors require review before merging. Every
code-signing request requires explicit approval by the approver.

## Build provenance

- Source repository: <https://github.com/jjfrancoms/aureus-client>
- Build workflow: [`.github/workflows/package.yml`](.github/workflows/package.yml)
- Official downloads: <https://github.com/jjfrancoms/aureus-client/releases>
- Privacy policy: [`PRIVACY.md`](PRIVACY.md)

The launcher verifies cryptographic hashes for Minecraft, Java and managed mod
downloads. It preserves a local manifest of Aureus-managed content and does not
silently delete files installed manually by the user.
