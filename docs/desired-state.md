# Desired-state planning

Desired-state mode composes explicit bytes with Threadmoth Core:

```text
observed bytes + desired bytes
          -> deterministic Diff Planner
          -> bounded disjoint ByteEdits
          -> Core guard / apply / verify / certify
```

The planner uses a deterministic Myers line-aware pass followed by deterministic byte-level refinement inside each changed coarse region. Long lines with distant small edits remain separate narrow regions where the bounded diff permits. It does not advertise “minimal edit,” merge independent hunks fuzzily, execute formatters, or silently fall back to a whole-file replacement when resource limits are exceeded.

Requests identify the workspace target in `file_path` and the desired state explicitly in the `desired_state` operation. Desired bytes are data, not a program. Effect budgets apply before commit through `max_changed_regions`, `max_changed_lines`, and `max_changed_bytes`.

Certificates expose `pre_hash`, `desired_hash`, `post_hash`, concrete changed ranges, and derived effect counts. The plan proof is `apply(observed, edits) == desired`; the commit proof is `readback == desired` and `post_hash == desired_hash`. If desired bytes equal observed bytes, the result is a clean no-op with matching hashes and no manufactured region.
