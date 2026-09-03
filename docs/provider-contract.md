# Provider contract

Providers may understand formats and propose edits; only Core commits.

A provider receives observed bytes and a typed operation. It must return bounded, sorted, non-overlapping byte edits or a typed refusal/error. It must not write files, invoke formatters, call a network, execute subprocesses, silently choose an ambiguous target, or claim an unverified preservation guarantee.

Core applies edits in memory, checks the resulting bytes, performs structural validation where applicable, guards source identity again, commits, and verifies landed bytes. Providers cannot bypass those gates.

Text is exact and byte-preserving outside matches. JSON v0.1 is strict and source-range based; it does not use whole-document serialization. TOML uses `toml_edit` only as a candidate generator and narrows the candidate; if representation drift cannot be bounded, it refuses.
