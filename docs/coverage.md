# Coverage model

Threadmoth classifies a discovered file before mutation. The level is an
honest description of what the provider can prove, not a promise to rewrite
the whole file.

| Level | Meaning | Examples |
| --- | --- | --- |
| `structured` | Addressable keys, values, sections or sequences with source-local edits | JSON, JSONC, TOML, YAML, INI, dotenv |
| `syntax` | Syntax nodes located by a parser; Core still owns all bytes written | JavaScript, TypeScript, Python, Rust, Go, C, C++, Bash, PowerShell, SQL, Java, C#, PHP, HCL, HTML, CSS, XML |
| `region` | Bounded document regions | Markdown |
| `exact` | No structural claim; exact, patch, pattern or desired-state routes remain explicit | unknown valid UTF-8, Dockerfile, Makefile |
| `opaque` | Unsupported encoding or binary-like input; text providers refuse | binary or invalid UTF-8 |

Discovery exposes `understanding_level`, `preservation_level`, detection
basis, fallback routes and alternatives. A structured request never silently
becomes an exact or regex request; the caller must explicitly choose the
weaker route.

The preservation levels are `unrelated_bytes`, `bounded_region`,
`explicit_desired_state` and `unavailable`. Parsers locate and validate;
Threadmoth Core authorizes, applies and certifies candidate bytes.

Kotlin, Swift and GNU Make syntax remain deferred. Java `.properties` remains
on exact text because its escaping, continuation and historical encoding rules
do not fit the current UTF-8 source-preserving contract.
