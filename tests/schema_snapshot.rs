use schemars::schema_for;
use threadmoth::protocol::{Certificate, Request, PROTOCOL_VERSION};

#[test]
fn exported_protocol_schema_has_a_golden_digest() {
    let value = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "Threadmoth 1.8.0 Protocol Schemas",
        "protocol_version": PROTOCOL_VERSION,
        "request": schema_for!(Request),
        "certificate": schema_for!(Certificate)
    });
    let rendered = serde_json::to_string_pretty(&value).unwrap();
    let digest = threadmoth::engine::compute_sha256(format!("{rendered}\n").as_bytes());
    assert_eq!(
        digest,
        "9c9613b224125098bf4f49f19efc50b465ee1f3c43286bd430e5e62147fe9ee7"
    );
}
