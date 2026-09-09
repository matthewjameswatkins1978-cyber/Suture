# Target registry

`src/target_registry.rs` is the canonical discovery registry. It owns target
IDs, aliases, extensions, exact filenames, provider ownership, coverage level,
preservation level and explicit fallback availability. Capability and inspect
responses use the same registry that detection uses.

Detection is deterministic: exact filename, registered extension, recognized
`.env` family, shebang evidence, validated UTF-8 content evidence, exact text
fallback, then opaque. Content evidence can report ambiguity but cannot grant
permission to select a provider. Registry tests reject duplicate IDs and
extension ownership collisions.

The Tree-sitter grammar table is keyed by these canonical IDs and supplies
parser implementations; it is not a second user-facing detection list.
