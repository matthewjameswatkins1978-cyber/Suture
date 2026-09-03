use schemars::schema_for;
use suture::protocol::{Certificate, Request, PROTOCOL_VERSION};

#[test]
fn exported_protocol_schema_has_a_golden_digest() {
    let value = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "Suture 1.1 Protocol Schemas",
        "protocol_version": PROTOCOL_VERSION,
        "request": schema_for!(Request),
        "certificate": schema_for!(Certificate)
    });
    let rendered = serde_json::to_string_pretty(&value).unwrap();
    let digest = suture::engine::compute_sha256(format!("{rendered}\n").as_bytes());
    assert_eq!(
        digest,
        "7049fbd9d7c666ff43da3d837b55ea82fa04118347dd69b16507ae6a22b63d02"
    );
}
