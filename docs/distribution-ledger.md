# Threadmoth distribution ledger

This is the durable record for Threadmoth discovery and distribution work. It records what was actually merged, packaged or published, not impressions or unverified reach. Update the snapshot after each meaningful external event.

## Snapshot

Date: 2026-09-09  
Repository: [matthewjameswatkins1978-cyber/Threadmoth](https://github.com/matthewjameswatkins1978-cyber/Threadmoth)  
Current source line on `main`: **Threadmoth 1.9.0**  
Current `main` merge for the 1.9 coverage/performance foundation: `946e7da0d59cc81fc4cacdfa6e821bb9812d391d`  
Stable published release: [Threadmoth 1.8.1](https://github.com/matthewjameswatkins1978-cyber/Threadmoth/releases/tag/v1.8.1)

Threadmoth 1.9 source is merged to `main`, but **v1.9.0 has not yet been tagged or published** at this snapshot. The published release page therefore correctly still shows 1.8.1.

The full compiler optimization matrix and separate PGO training/validation are follow-up performance work, not blockers for publishing the portable and modern 1.9.0 artifacts. Hard publication gates are the protected `main` checks, the release workflow's cross-platform build/package/runtime checks, complete checksum/manifest generation, and the published-release updater exercise. Until those hard gates are complete, do not describe v1.9.0 as a published release.

The repository slug is `/Threadmoth`; the old `/Suture` URL may redirect but is not the canonical identity.

## What is merged for 1.9

The 1.9 source line includes:

- canonical target/coverage registry;
- structured/syntax/region/exact/opaque discovery metadata;
- Java, C#, PHP, and HCL / Terraform syntax targeting through the shared Tree-sitter engine;
- nested conservative YAML targeting;
- source-preserving INI-style targeting;
- portable, `x86-64-v3` modern, and local-native build tooling;
- updater flavour selection support;
- current 1.9 coverage, performance, protocol and agent documentation.

The release remains correctness-gated. Local 1.9 performance smoke evidence recorded in the repository reports `wrong_applied: 0`, but it is not a substitute for the outstanding release matrix.

## Current published artifacts

The latest published release at this snapshot remains 1.8.1, with release artifacts for:

- Windows x86-64;
- Linux x86-64;
- macOS Apple Silicon;
- macOS x86-64;
- release manifest and per-platform checksum assets.

The 1.9 release workflow is prepared to add explicit Windows/Linux `x86-64-v3` modern artifacts alongside portable builds once v1.9.0 is actually released.

## Shipped repository integrations

| Target | Artifact | Status | Validation |
|---|---|---|---|
| Portable Agent Skills | [`skills/threadmoth/SKILL.md`](../skills/threadmoth/SKILL.md) | Shipped | Shared skill is versioned for 1.9 source and discovers the installed runtime before use |
| Claude Code | [`.claude-plugin/plugin.json`](../.claude-plugin/plugin.json) plus shared skill | Shipped | Standard plugin layout; still depends on user-installed `threadmoth` executable |
| Gemini CLI | [`gemini-extension.json`](../gemini-extension.json), [`GEMINI.md`](../GEMINI.md), shared skill | Shipped | Root manifest/context/skill layout present; runtime capabilities are discovered locally |
| Antigravity | [`plugin.json`](../plugin.json) plus shared skill | Packaged; field validation still incomplete | Authentication and skill discovery were verified locally; treat end-to-end mutation/refusal recovery as unverified until a clean reproducible field run completes |

All adapters require the user-installed `threadmoth` executable on `PATH`. They do not install a binary, grant filesystem permissions, silently start a server, or replace other editing tools.

## Historical 1.7 distribution work

Threadmoth 1.7 added important distribution foundations that remain relevant:

- MCP exposed more read-only discovery/recovery surfaces;
- deterministic guarded candidate selection gave agents a safe ambiguity-recovery route;
- refusal recovery preserved provider semantics rather than silently dropping to generic text edits;
- filename/provider detection covered more ordinary repository filenames;
- release automation added macOS Apple Silicon and Intel alongside Windows/Linux;
- release workflow refused to publish a version tag whose commit was not already contained in `main`.

These are historical milestones, not the current capability list. Use [Coverage](coverage.md) and runtime `capabilities` for 1.9.

## Existing outreach

The following entries are operational history. Their state should not be interpreted as current traffic or adoption without a fresh check.

| Target | Fit/action | Link | Status / next action |
|---|---|---|---|
| GitHub field testers | Canonical feedback route | [Issue #24](https://github.com/matthewjameswatkins1978-cyber/Threadmoth/issues/24) | Point testers at current skill, coverage docs and agent challenge |
| DEV | Technical article | [Article](https://dev.to/matmusmeows/threadmoth-a-deterministic-source-preserving-mutation-boundary-for-ai-coding-agents-2a2g) | Existing article; publish follow-ups only when there is useful new evidence |
| Reddit r/ChatGPTCoding | Existing outreach | [Post/comment](https://www.reddit.com/r/ChatGPTCoding/comments/1w372gj/comment/p7ui0gt/) | Do not duplicate; answer genuine replies |
| Reddit r/opensource | Existing outreach | [Post](https://www.reddit.com/r/opensource/comments/1w7fivf/threadmoth_deterministic_file_mutation_for_ai/) | Do not duplicate; answer genuine replies |
| Reddit r/rust | Existing outreach | [Post](https://www.reddit.com/r/rust/comments/1w7fjsu/threadmoth_sourcepreserving_structural_file/) | Do not duplicate; answer genuine replies |
| Reddit r/SideProject | Existing outreach | [Post](https://www.reddit.com/r/SideProject/comments/1w7fsos/i_built_threadmoth_to_make_aiassisted_file_edits/) | Do not duplicate; answer genuine replies |
| Rust Users Forum | Showcase attempt | [Forum](https://users.rust-lang.org/) | Check public/moderation status before follow-up |
| Hacker News | Show HN | — | Earlier attempt was gated by account/community conditions; do not work around moderation |
| Cline / other agent communities | Potentially useful | — | Use approved community routes and avoid generic duplicate promotion |

## Planned distribution work

| Target | Fit | Gate |
|---|---|---|
| Threadmoth v1.9.0 GitHub release | Highest | Complete updater/runtime acceptance, then tag/publish from verified `main`; PGO remains follow-up work |
| Claude Code official directory | High | Authenticated submission plus live integration validation |
| Gemini CLI gallery | High | Manifest review and live install validation before gallery submission |
| skills.sh | High | Public skill can be installed by users; do not infer telemetry before observed use |
| Codex Discussions / agent communities | Useful | Post only if venue rules and actual project evidence justify it |

## Slug audit

The repository uses `/Threadmoth` as its canonical slug. Current package metadata, extension sources, release links, and documentation should use that identity.
