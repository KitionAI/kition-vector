//! `kit.json` publishes the engine's editor state as `engine.editorState` (Kition platform K23),
//! where a host that does not run this crate reads it (to hand an agent `inspect()` without it):
//! it must be the engine's own list, in its order.

#[test]
fn kit_json_declares_the_engines_editor_state() {
    let manifest: serde_json::Value = serde_json::from_str(include_str!("../../../kit.json")).unwrap();
    assert_eq!(
        manifest["engine"]["editorState"],
        serde_json::json!(kition_vector_kit::EDITOR_STATE),
        "kit.json engine.editorState is not the engine's EDITOR_STATE"
    );
}
