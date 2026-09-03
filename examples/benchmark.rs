use std::time::Instant;
use suture::pipeline::execute_request;
use suture::protocol::{Cardinality, OperationPayload, Outcome, Request, PROTOCOL_VERSION};
use suture::provider::text::TextOperation;
use suture::workspace::Workspace;

fn main() {
    println!("case,bytes,iterations,wrong_applied,avg_us,certificate_bytes");
    for (name, mut bytes) in [
        ("tiny", b"prefix FINDME suffix\n".to_vec()),
        ("config_30k", vec![b'a'; 30_000]),
        ("text_1m", vec![b'a'; 1_000_000]),
    ] {
        if name != "tiny" {
            bytes.extend_from_slice(b"\nFINDME\n");
        }
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("input.txt");
        std::fs::write(&path, &bytes).unwrap();
        let workspace = Workspace::new(temp.path()).unwrap();
        let request = Request {
            version: PROTOCOL_VERSION.into(),
            file_path: "input.txt".into(),
            namespace: Default::default(),
            expected_pre_hash: None,
            cardinality: Cardinality::ExactlyOne,
            operation: OperationPayload::Text(TextOperation::Replace {
                target: "FINDME".into(),
                replacement: "FOUND".into(),
            }),
        };
        let iterations = 25;
        let start = Instant::now();
        let mut wrong = 0;
        let mut cert_size = 0;
        for _ in 0..iterations {
            let cert = execute_request(&workspace, &request, true);
            if cert.outcome != Outcome::Applied || cert.post_hash.is_none() {
                wrong += 1;
            }
            cert_size = serde_json::to_vec(&cert).unwrap().len();
        }
        let avg_us = start.elapsed().as_secs_f64() * 1_000_000.0 / iterations as f64;
        println!(
            "{name},{},{iterations},{wrong},{avg_us:.1},{cert_size}",
            bytes.len()
        );
    }
}
