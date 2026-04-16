# Contributing to taktflow-opensovd

Thank you for your interest in contributing. This document explains how to
contribute effectively and what standards we enforce.

## Prerequisites

Before contributing, you must:

1. **Sign the Eclipse Contributor Agreement (ECA)** at
   https://www.eclipse.org/legal/ECA.php. All commits must come from an
   ECA-signed email address.
2. **Read the coding standards** in [docs/CODING-STANDARDS.md](docs/CODING-STANDARDS.md).
3. **Read the test strategy** in [docs/TEST-STRATEGY.md](docs/TEST-STRATEGY.md).

## Development setup

See [docs/DEVELOPER-GUIDE.md](docs/DEVELOPER-GUIDE.md) for build prerequisites,
toolchain setup, and how to run the test suite.

## Branching model

| Branch | Purpose |
|--------|---------|
| `main` | Stable, reviewed code. All PRs target this branch. |
| `feature/*` | Feature development branches. |
| `auto/line-a/*` | Automated Line A (Rust/opensovd-core) work. |
| `auto/line-b/*` | Automated Line B (embedded firmware) work. |
| `fix/*` | Bug fix branches. |

## Commit conventions

Every commit message must follow this format:

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types:** `feat`, `fix`, `docs`, `test`, `refactor`, `ci`, `chore`, `perf`.

**Scopes:** Use the crate or component name (e.g., `sovd-server`, `sovd-dfm`,
`cda`, `fault-lib`, `odx-converter`, `ci`).

Examples:
```
feat(sovd-gateway): add parallel fan-out for component discovery
fix(sovd-dfm): prevent deadlock when operation cycle ends during fault ingest
test(integration): add Phase 5 HIL concurrent tester scenario
docs(adr): ADR-0018 lock budget for DFM shared state
```

## Pull request process

1. **One logical change per PR.** Do not bundle unrelated changes.
2. **All CI checks must pass:** clippy (pedantic, deny warnings), rustfmt,
   cargo-deny (license + advisory audit), and the full test suite.
3. **Include tests** for new functionality. Bug fixes must include a regression test.
4. **Update documentation** if your change affects public API, architecture, or
   deployment.
5. **SPDX headers required** on all new source files. See
   [docs/CODING-STANDARDS.md](docs/CODING-STANDARDS.md) for the exact format.

### Safety-critical changes

Any change that touches the safety boundary (Fault Library API, routine
interlocks, ASIL-related code paths) requires:

- Explicit mention in the PR description with rationale.
- Review and sign-off from a safety engineer.
- MISRA C:2012 static analysis pass (for embedded C code).

See [docs/SAFETY-CONCEPT.md](docs/SAFETY-CONCEPT.md) for the full safety
architecture.

## Code review expectations

- Reviewers check correctness, safety implications, test coverage, and
  adherence to coding standards.
- Authors are expected to respond to review comments within 2 working days.
- Approve requires at least one maintainer sign-off.

## Reporting issues

Use GitHub Issues. Include:
- Component affected (crate name or subsystem).
- Steps to reproduce.
- Expected vs. actual behavior.
- Environment (OS, Rust version, hardware if HIL-related).

## License

By contributing, you agree that your contributions will be licensed under the
Apache License 2.0. See [LICENSE](LICENSE).
