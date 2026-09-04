# Syntax targeting

Threadmoth uses Tree-sitter to identify source-node boundaries and validate candidate syntax. It does not unparse, pretty-print, serialize, or regenerate an AST.

## AST-grounded

AST-grounded targeting guarantees exact source text, a syntax-node boundary, and the required cardinality. The target may be structurally real without having a caller-requested grammatical role. Identical text in a string or comment is not silently treated as the same code target.

## AST-typed

AST-typed targeting additionally requires the caller to provide `node_kind`. It guarantees exact source text, a syntax-node boundary, the exact requested node kind, and the required cardinality. Committed requests do not infer or strengthen `node_kind`; discovery may suggest a kind, but the caller must choose it explicitly.

The same shared engine serves the statically compiled programming-language and web-format registries. The 1.5 web envelope is deliberately structural: HTML, CSS, and XML do not claim deep embedded-script/style or DOM semantic editing.
