---
name: Unexpected mutation or preservation problem
about: Report collateral changes, surprising writes, or preservation failures
title: "Unexpected mutation: "
labels: ""
assignees: ""
---

## Stop and preserve evidence

If possible, keep the pre-image, post-image, request, certificate, and relevant Git diff. Do not post secrets or proprietary source.

## Summary

What changed that you did not authorize or expect?

## Environment

- Threadmoth version:
- OS:
- Shell:
- Provider/language:
- Coding agent/tool, if any:

## Request

Paste the smallest sanitized request that reproduces the problem.

```json
{}
```

## Expected change

Describe the exact intended mutation.

## Actual change

Describe the unexpected bytes, formatting, comments, line endings, paths, or files affected.

## Certificate

Paste the smallest useful sanitized certificate or summary.

```text
paste evidence here
```

## Diff

If safe to share, include a reduced diff that demonstrates the collateral change.

```diff

```

## Reproducibility

- [ ] Reproduces every time
- [ ] Intermittent
- [ ] Happened once after a crash/interruption
- [ ] Involved transaction recovery

## Did Threadmoth report APPLIED, NO_CHANGE, REFUSED, or FAILED?

Outcome:

## Anything else touching the same file?

Editors, formatters, watchers, build tools, sync software, or another agent process: