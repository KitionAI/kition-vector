//! C8-vector-engine: `command.batch` is registered as the write it is (its steps edit), not as a
//! query, and is journaled once, with what its steps noted (issue-c8-engine-batch-write).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kition_vector_engine::{Session, cmd::find_command};
use serde_json::json;

#[test]
fn a_batch_is_registered_as_a_write() {
    let spec = find_command("command.batch").unwrap();
    assert!(spec.journal, "command.batch is a write, journaled like one");
}

#[test]
fn a_batch_is_journaled_once() {
    let mut s = Session::new();
    s.execute("file.new", &json!({})).unwrap();
    let start = s.journal.len();
    s.execute("command.batch", &json!({"commands": [{"command": "shape.rectangle", "params": {"x": 0, "y": 0, "width": 5, "height": 5}}]})).unwrap();
    let entries: Vec<&String> = s.journal[start..].iter().map(|(id, _)| id).collect();
    assert_eq!(entries, vec!["command.batch"]);
}
