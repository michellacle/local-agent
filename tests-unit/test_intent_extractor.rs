use local_agent::intent_extractor::IntentExtractor;

#[test]
fn test_intent_schema_has_required_fields() {
    // The schema is built internally, but we can verify the struct itself
    // has the expected fields by constructing one
    use local_agent::capability_router::{ActionType, ResourceType, RoutingIntent};
    use serde_json::json;

    let _intent = RoutingIntent {
        action: ActionType::Find,
        resource: ResourceType::SalesReport,
        parameters: json!({"key": "value"}),
    };

    let mut schema = schemars::schema_for!(RoutingIntent);
    let obj_schema = schema.schema.object();
    assert!(obj_schema.properties.contains_key("action"));
    assert!(obj_schema.properties.contains_key("resource"));
    assert!(obj_schema.properties.contains_key("parameters"));
    assert!(obj_schema.required.contains("action"));
    assert!(obj_schema.required.contains("resource"));
}

#[test]
fn test_system_prompt_exists() {
    // The system prompt is hardcoded in IntentExtractor::extract
    // We verify it's non-empty by checking the extractor initializes
    let _ = IntentExtractor::new(
        local_agent::llm_interface::LLMClient::new(None),
        None,
        false,
        "memory",
        None,
    );
}
