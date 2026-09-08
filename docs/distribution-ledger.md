# Threadmoth distribution ledger

This is the durable record for Threadmoth discovery work. It records what was actually shipped or published, not impressions or unverified reach. Update the snapshot and next action after each meaningful external event.

## Snapshot

Date: 2026-09-08  
Stable published release: [Threadmoth 1.7.1](https://github.com/matthewjameswatkins1978-cyber/Threadmoth/releases/tag/v1.7.1)
Repository: [matthewjameswatkins1978-cyber/Threadmoth](https://github.com/matthewjameswatkins1978-cyber/Threadmoth)
Repository slug: `/Threadmoth`; the old `/Suture` URL redirects to the new canonical repository.

Threadmoth 1.7.1 is published with release artifacts for Windows x86-64, Linux x86-64, macOS Apple Silicon, and macOS x86-64. `main` now contains the 1.7.1 release line and is the release branch of record.

The earlier adoption snapshot reported 0 stars, 0 forks, and 0 watchers, with release asset download counters mostly in the 1–2 range. Those figures are historical observations rather than live installation counts and should not be treated as current metrics without a fresh GitHub check.

## Shipped in the repository

| Target | Artifact | Status | Validation |
|---|---|---|---|
  | Portable Agent Skills | [`skills/threadmoth/SKILL.md`](../skills/threadmoth/SKILL.md) | Shipped | Agent Skills frontmatter and progressive-disclosure requirements checked |
  | Claude Code | [`.claude-plugin/plugin.json`](../.claude-plugin/plugin.json) plus the shared skill | Shipped | Standard plugin layout and manifest shape checked against Claude Code docs |
  | Gemini CLI | [`gemini-extension.json`](../gemini-extension.json), [`GEMINI.md`](../GEMINI.md), shared skill | Shipped | Root manifest, context file, and bundled skill layout checked against Gemini CLI docs |
  | Antigravity | [`plugin.json`](../plugin.json) plus the shared skill | Packaged; discovery and authentication verified; live mutation pending | `agy 1.1.27`; Google OAuth authenticated as `easyennui@gmail.com`; agent loaded the skill and selected Threadmoth, but the run entered unsafe Git investigation; the destructive action was refused. Test A changed only the requested `app.json` value, but tool provenance is inconclusive; refusal/recovery Test B is pending and Antigravity is not live-verified |

All adapters require the user-installed `threadmoth` executable on PATH. They do not install a binary, start an MCP server, grant permissions, or silently replace other editing tools.

## Shipped in 1.7

- MCP exposes more of Threadmoth's existing read-only discovery and recovery surfaces.
- Deterministic guarded candidate selection gives agents a safe way to choose one genuinely ambiguous occurrence against an exact observed file state.
- Refusal recovery preserves provider semantics rather than falling back to generic text edits.
- Filename/provider detection covers more common real-world filenames without fuzzy language guessing.
- Release automation adds macOS Apple Silicon and Intel targets alongside Windows and Linux.
- Agent documentation includes a short discovery → preview → refusal recovery → commit workflow.
- The release workflow now refuses to publish a version tag whose commit is not already contained in `main`, and package version smoke checks derive the expected version from the tag rather than hard-coding a release number.

## Existing outreach

| Target | Fit/action | Link | Status / moderation | Replies or evidence | Next action |
|---|---|---|---|---|---|
| GitHub field testers | Canonical feedback route | [Issue #24](https://github.com/matthewjameswatkins1978-cyber/Threadmoth/issues/24) | Open | No substantive external tester report recorded at the earlier snapshot | Point agents to the skill, challenge, and issue |
| DEV | Technical article | [Article](https://dev.to/matmusmeows/threadmoth-a-deterministic-source-preserving-mutation-boundary-for-ai-coding-agents-2a2g) | Live | No measured external feedback recorded | Publish the refusal/ambiguity follow-up only after review |
| Reddit r/ChatGPTCoding | Weekly self-promotion | [Post/comment](https://www.reddit.com/r/ChatGPTCoding/comments/1w372gj/comment/p7ui0gt/) | Existing outreach | No substantive tester report | Do not duplicate; answer genuine replies |
| Reddit r/opensource | Project introduction | [Post](https://www.reddit.com/r/opensource/comments/1w7fivf/threadmoth_deterministic_file_mutation_for_ai/) | Existing outreach | No substantive tester report | Do not duplicate; answer genuine replies |
| Reddit r/rust | Technical project introduction | [Post](https://www.reddit.com/r/rust/comments/1w7fjsu/threadmoth_sourcepreserving_structural_file/) | Existing outreach | No substantive tester report | Do not duplicate; answer genuine replies |
| Reddit r/SideProject | Build story | [Post](https://www.reddit.com/r/SideProject/comments/1w7fsos/i_built_threadmoth_to_make_aiassisted_file_edits/) | Existing outreach | No substantive tester report | Do not duplicate; answer genuine replies |
| Rust Users Forum | Showcase | [Forum](https://users.rust-lang.org/) | Accepted subject to moderator approval; public topic URL not recorded | No substantive tester report | Check for approval before any follow-up |
| Hacker News | Show HN | — | Rejected before creation because the contributor/account was newer or unfamiliar | No discussion created | Do not workaround moderation; revisit after real usage evidence |
| Cline | Community project sharing | — | Held because the relevant route required moderator approval | No post | Seek the approved route only if permission is granted |
| Claude community | Showcase | — | Held because the route required substantial karma and clear Claude-built framing | No post | Submit the plugin through Anthropic's official form when eligible |

## Planned, not yet published

| Target | Fit | Gate |
|---|---|---|
| Claude Code official directory | High: the repository now has a minimal plugin adapter | Authenticated submission through the official Claude plugin form; live Claude validation still pending |
| Gemini CLI gallery | High: the root manifest and `gemini-cli-extension` topic route are available | Add the topic only with the repository owner's approval after manifest review; validate with a live Gemini CLI install |
| skills.sh | High: the skill is a public GitHub skill with valid `SKILL.md` metadata | A user must run `npx skills add ...`; install telemetry is not available before that |
| Codex Discussions | High: technical show-and-tell and break-me challenge | Confirm Discussions is enabled and post under the community rules |
| Cursor / Cline / other agent communities | Potentially useful | Follow each venue's current sharing and moderation rules; no duplicate generic promotion |

## Slug audit

The repository now uses `/Threadmoth` as its canonical slug; the old `/Suture` URL redirects to it. Current documentation, package metadata, extension sources, and release links use the new slug.
