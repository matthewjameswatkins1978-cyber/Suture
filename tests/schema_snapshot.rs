use schemars::schema_for;
use suture::protocol::{Certificate, Request, PROTOCOL_VERSION};

#[test]
fn exported_protocol_schema_has_a_golden_digest() {
    let value = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "Suture v0.1 Protocol Schemas",
        "protocol_version": PROTOCOL_VERSION,
        "request": schema_for!(Request),
        "certificate": schema_for!(Certificate)
    });
    let rendered = serde_json::to_string_pretty(&value).unwrap();
    let digest = suture::engine::compute_sha256(format!("{rendered}\n").as_bytes());
    assert_eq!(
        digest,
        "ddac5889563ba66ed0274ff109a5b9fda5a46ac71d6bb26b86096d6051ac9dbb"
    );
}
