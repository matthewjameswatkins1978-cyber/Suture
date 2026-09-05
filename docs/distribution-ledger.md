# Threadmoth distribution ledger

This is the durable record for Threadmoth discovery work. It records what was
actually shipped or published, not impressions or unverified reach. Update the
snapshot and next action after each meaningful external event.

## Snapshot

Date: 2026-09-05  
Stable release: [Threadmoth 1.5.1](https://github.com/matthewjameswatkins1978-cyber/Suture/releases/tag/v1.5.1)  
Repository: [matthewjameswatkins1978-cyber/Suture](https://github.com/matthewjameswatkins1978-cyber/Suture)  
Repository slug: still `/Suture`; no slug rename is part of this distribution pass.

At snapshot time GitHub reported 0 stars, 0 forks, and 0 watchers. The v1.5.1
release and CI workflows were green. Release asset download counters were
small (mostly 1–2 per asset) and are not treated as unique installations.

## Shipped in the repository

| Target | Artifact | Status | Validation |
|---|---|---|---|
| Portable Agent Skills | [`skills/threadmoth/SKILL.md`](../skills/threadmoth/SKILL.md) | Shipped in this change | Agent Skills frontmatter and progressive-disclosure requirements checked |
| Claude Code | [`.claude-plugin/plugin.json`](../.claude-plugin/plugin.json) plus the shared skill | Shipped in this change | Standard plugin layout and manifest shape checked against Claude Code docs |
| Gemini CLI | [`gemini-extension.json`](../gemini-extension.json), [`GEMINI.md`](../GEMINI.md), shared skill | Shipped in this change | Root manifest, context file, and bundled skill layout checked against Gemini CLI docs |

All adapters require the user-installed `threadmoth` executable on PATH. They
do not install a binary, start an MCP server, grant permissions, or silently
replace other editing tools.

## Existing outreach

| Target | Fit/action | Link | Status / moderation | Replies or evidence | Next action |
|---|---|---|---|---|---|
| GitHub field testers | Canonical feedback route | [Issue #24](https://github.com/matthewjameswatkins1978-cyber/Suture/issues/24) | Open | No substantive external tester report at snapshot | Point agents to the skill, challenge, and issue |
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

Keeping `/Suture` is technically functional but creates a discoverability and
trust mismatch for a project now called Threadmoth. Renaming the repository is
not included here because it would touch clone URLs, cargo-install examples,
release links, package metadata, existing outreach, and future extension
install sources. Revisit the slug after the adapter and field-test links have
settled, with redirects and a link audit prepared first.
