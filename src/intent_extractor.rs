use crate::capability_router::{ActionType, AgentRouter, ResourceType, RouteResult, RoutingIntent};
use crate::llm_interface::{EmbeddingClientTrait, LLMClientTrait};
use crate::semantic_cache::SemanticCache;

/// Orchestrates the full pipeline: extracts intent from natural language, checks the semantic cache, and routes to the correct agent.
pub struct IntentExtractor {
    /// LLM client used for intent extraction.
    client: Box<dyn LLMClientTrait>,
    /// Embedding client for generating query vectors (only when cache is enabled).
    embed_client: Option<Box<dyn EmbeddingClientTrait>>,
    /// Router that maps resolved intents to agent names.
    router: AgentRouter,
    /// Optional semantic cache for bypassing the LLM on similar queries.
    pub cache: Option<Box<dyn SemanticCache>>,
}

impl IntentExtractor {
    pub fn new(
        client: Box<dyn LLMClientTrait>,
        embed_client: Option<Box<dyn EmbeddingClientTrait>>,
        cache: Option<Box<dyn SemanticCache>>,
    ) -> Self {
        Self {
            client,
            embed_client,
            router: AgentRouter::new(),
            cache,
        }
    }

    pub fn extract(&mut self, user_input: &str) -> Result<RoutingIntent, String> {
        if let (Some(cache), Some(embed_client)) = (&mut self.cache, &self.embed_client) {
            let embedding = embed_client.embed(user_input)?;
            if let Some(cached) = cache.lookup(&embedding) {
                println!("Cache HIT (threshold {})", cache.threshold());
                return Ok(cached);
            }
        }

        let intent_schema = Self::build_intent_schema();
        let messages = vec![serde_json::json!({
            "role": "user",
            "content": format!("Extract the intent from this request:\n\n{user_input}")
        })];

        let system_prompt = r#"You are an intent extraction assistant.

Given a natural language request, extract:
- action: one of find, analyze, create
- resource: one of sales_report, server_log, document, loan_application
- parameters: any relevant details as a dict

If the request doesn't match any action/resource combination,
still extract the closest action and resource you can infer."#;

        let result: serde_json::Value =
            match self
                .client
                .structured_chat(messages.clone(), intent_schema, Some(system_prompt))
            {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Structured output failed ({e}), trying plain text fallback...");
                    let raw = self.client.chat(messages, Some(system_prompt), None)?;
                    serde_json::from_str(&raw).map_err(|e| format!("Fallback parse failed: {e}"))?
                }
            };

        let action: ActionType = serde_json::from_value(result["action"].clone())
            .map_err(|e| format!("Invalid action: {e}"))?;
        let resource: ResourceType = serde_json::from_value(result["resource"].clone())
            .map_err(|e| format!("Invalid resource: {e}"))?;
        let parameters = result
            .get("parameters")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        let intent = RoutingIntent {
            action,
            resource,
            parameters,
        };

        if let (Some(cache), Some(embed_client)) = (&mut self.cache, &self.embed_client) {
            let embedding = embed_client.embed(user_input)?;
            cache.store(user_input, &embedding, &intent);
            println!("Cache MISS → stored for future hits");
        }

        Ok(intent)
    }

    pub fn process(&mut self, user_input: &str) -> Result<RouteResult, String> {
        let intent = self.extract(user_input)?;
        println!(
            "Extracted intent: action={}, resource={}, params={}",
            serde_json::to_value(&intent.action).unwrap(),
            serde_json::to_value(&intent.resource).unwrap(),
            intent.parameters
        );
        Ok(self.router.route_request(&intent))
    }

    fn build_intent_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["find", "analyze", "create"],
                    "description": "The action to perform."
                },
                "resource": {
                    "type": "string",
                    "enum": ["sales_report", "server_log", "document", "loan_application"],
                    "description": "The resource to act on."
                },
                "parameters": {
                    "type": "object",
                    "description": "Additional parameters extracted from the request."
                }
            },
            "required": ["action", "resource"]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability_router::{ActionType, ResourceType, RoutingIntent};
    use crate::llm_interface::{EmbeddingClientTrait, LLMClientTrait};
    use crate::semantic_cache::{InMemorySemanticCache, SemanticCache};
    use serde_json::json;

    struct MockLLMClient {
        structured_response: Result<serde_json::Value, String>,
        chat_response: Result<String, String>,
    }

    impl MockLLMClient {
        fn new_valid_intent() -> Self {
            Self {
                structured_response: Ok(json!({
                    "action": "find",
                    "resource": "sales_report",
                    "parameters": {"quarter": "Q1"}
                })),
                chat_response: Err("should not reach chat fallback".into()),
            }
        }

        fn new_error() -> Self {
            Self {
                structured_response: Err("LLM error".into()),
                chat_response: Err("chat also fails".into()),
            }
        }

        fn new_fallback_valid() -> Self {
            Self {
                structured_response: Err("structured fails, try fallback".into()),
                chat_response: Ok(json!({
                    "action": "analyze",
                    "resource": "server_log",
                    "parameters": {"date": "2024-01-01"}
                })
                .to_string()),
            }
        }

        fn new_invalid_action() -> Self {
            Self {
                structured_response: Ok(json!({
                    "action": "unknown_action",
                    "resource": "sales_report",
                    "parameters": {}
                })),
                chat_response: Err("should not reach chat fallback".into()),
            }
        }

        fn new_invalid_resource() -> Self {
            Self {
                structured_response: Ok(json!({
                    "action": "find",
                    "resource": "unknown_resource",
                    "parameters": {}
                })),
                chat_response: Err("should not reach chat fallback".into()),
            }
        }
    }

    impl LLMClientTrait for MockLLMClient {
        fn chat(
            &self,
            _messages: Vec<serde_json::Value>,
            _system_prompt: Option<&str>,
            _response_format: Option<serde_json::Value>,
        ) -> Result<String, String> {
            self.chat_response.clone()
        }

        fn structured_chat(
            &self,
            _messages: Vec<serde_json::Value>,
            _schema: serde_json::Value,
            _system_prompt: Option<&str>,
        ) -> Result<serde_json::Value, String> {
            self.structured_response.clone()
        }
    }

    struct MockEmbeddingClient {
        embedding: Vec<f64>,
    }

    impl MockEmbeddingClient {
        fn new(embedding: Vec<f64>) -> Self {
            Self { embedding }
        }
    }

    impl EmbeddingClientTrait for MockEmbeddingClient {
        fn embed(&self, _text: &str) -> Result<Vec<f64>, String> {
            Ok(self.embedding.clone())
        }
    }

    #[test]
    fn test_intent_schema_has_required_fields() {
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
        let client: Box<dyn LLMClientTrait> = Box::new(MockLLMClient::new_valid_intent());
        let _ = IntentExtractor::new(client, None, None::<Box<dyn SemanticCache>>);
    }

    #[test]
    fn test_extract_valid_intent() {
        let client: Box<dyn LLMClientTrait> = Box::new(MockLLMClient::new_valid_intent());
        let mut extractor = IntentExtractor::new(client, None, None::<Box<dyn SemanticCache>>);

        let intent = extractor.extract("Find the Q1 sales report").unwrap();
        assert_eq!(intent.action, ActionType::Find);
        assert_eq!(intent.resource, ResourceType::SalesReport);
        assert_eq!(intent.parameters["quarter"], "Q1");
    }

    #[test]
    fn test_extract_llm_error_propagates() {
        let client: Box<dyn LLMClientTrait> = Box::new(MockLLMClient::new_error());
        let mut extractor = IntentExtractor::new(client, None, None::<Box<dyn SemanticCache>>);

        let result = extractor.extract("Find the Q1 sales report");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("chat also fails"));
    }

    #[test]
    fn test_extract_fallback_to_chat() {
        let client: Box<dyn LLMClientTrait> = Box::new(MockLLMClient::new_fallback_valid());
        let mut extractor = IntentExtractor::new(client, None, None::<Box<dyn SemanticCache>>);

        let intent = extractor.extract("Analyze the server log").unwrap();
        assert_eq!(intent.action, ActionType::Analyze);
        assert_eq!(intent.resource, ResourceType::ServerLog);
        assert_eq!(intent.parameters["date"], "2024-01-01");
    }

    #[test]
    fn test_extract_invalid_action() {
        let client: Box<dyn LLMClientTrait> = Box::new(MockLLMClient::new_invalid_action());
        let mut extractor = IntentExtractor::new(client, None, None::<Box<dyn SemanticCache>>);

        let result = extractor.extract("Find the Q1 sales report");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid action"));
    }

    #[test]
    fn test_extract_invalid_resource() {
        let client: Box<dyn LLMClientTrait> = Box::new(MockLLMClient::new_invalid_resource());
        let mut extractor = IntentExtractor::new(client, None, None::<Box<dyn SemanticCache>>);

        let result = extractor.extract("Find the Q1 sales report");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid resource"));
    }

    #[test]
    fn test_extract_cache_hit_returns_cached_intent() {
        let client: Box<dyn LLMClientTrait> = Box::new(MockLLMClient::new_valid_intent());

        let embedding = vec![1.0, 0.0, 0.0];
        let cached_intent = RoutingIntent {
            action: ActionType::Create,
            resource: ResourceType::Document,
            parameters: json!({"topic": "audit"}),
        };

        let mut cache = InMemorySemanticCache::new(0.9, 100);
        cache.store("Create an audit document", &embedding, &cached_intent);

        let embed_client: Box<dyn EmbeddingClientTrait> =
            Box::new(MockEmbeddingClient::new(embedding.clone()));

        let mut extractor = IntentExtractor::new(client, Some(embed_client), Some(Box::new(cache)));

        let intent = extractor.extract("Create an audit document").unwrap();
        assert_eq!(intent.action, ActionType::Create);
        assert_eq!(intent.resource, ResourceType::Document);
        assert_eq!(intent.parameters["topic"], "audit");
    }

    #[test]
    fn test_extract_cache_miss_calls_llm_and_stores() {
        let client: Box<dyn LLMClientTrait> = Box::new(MockLLMClient::new_valid_intent());

        let mut cache = InMemorySemanticCache::new(0.9, 100);
        cache.store(
            "Different query",
            &[0.0, 1.0, 0.0],
            &RoutingIntent {
                action: ActionType::Find,
                resource: ResourceType::ServerLog,
                parameters: json!({}),
            },
        );

        let embedding = vec![1.0, 0.0, 0.0];
        let embed_client: Box<dyn EmbeddingClientTrait> =
            Box::new(MockEmbeddingClient::new(embedding.clone()));

        let mut extractor = IntentExtractor::new(client, Some(embed_client), Some(Box::new(cache)));

        let intent = extractor.extract("Find the Q1 sales report").unwrap();
        assert_eq!(intent.action, ActionType::Find);
        assert_eq!(intent.resource, ResourceType::SalesReport);
    }

    #[test]
    fn test_extract_no_cache_calls_llm() {
        let client: Box<dyn LLMClientTrait> = Box::new(MockLLMClient::new_valid_intent());
        let mut extractor = IntentExtractor::new(client, None, None::<Box<dyn SemanticCache>>);

        let intent = extractor.extract("Find the Q1 sales report").unwrap();
        assert_eq!(intent.action, ActionType::Find);
        assert_eq!(intent.resource, ResourceType::SalesReport);
    }

    #[test]
    fn test_extract_missing_parameters_defaults_to_empty() {
        struct MockLLMClientNoParams;

        impl LLMClientTrait for MockLLMClientNoParams {
            fn chat(
                &self,
                _messages: Vec<serde_json::Value>,
                _system_prompt: Option<&str>,
                _response_format: Option<serde_json::Value>,
            ) -> Result<String, String> {
                Err("should not reach chat fallback".into())
            }

            fn structured_chat(
                &self,
                _messages: Vec<serde_json::Value>,
                _schema: serde_json::Value,
                _system_prompt: Option<&str>,
            ) -> Result<serde_json::Value, String> {
                Ok(json!({
                    "action": "find",
                    "resource": "document"
                }))
            }
        }

        let client: Box<dyn LLMClientTrait> = Box::new(MockLLMClientNoParams);
        let mut extractor = IntentExtractor::new(client, None, None::<Box<dyn SemanticCache>>);

        let intent = extractor.extract("Find the document").unwrap();
        assert_eq!(intent.action, ActionType::Find);
        assert_eq!(intent.resource, ResourceType::Document);
        assert_eq!(intent.parameters, json!({}));
    }

    #[test]
    fn test_extract_fallback_parse_error() {
        struct MockLLMClientFallbackParseError;

        impl LLMClientTrait for MockLLMClientFallbackParseError {
            fn chat(
                &self,
                _messages: Vec<serde_json::Value>,
                _system_prompt: Option<&str>,
                _response_format: Option<serde_json::Value>,
            ) -> Result<String, String> {
                Ok("not valid json at all".into())
            }

            fn structured_chat(
                &self,
                _messages: Vec<serde_json::Value>,
                _schema: serde_json::Value,
                _system_prompt: Option<&str>,
            ) -> Result<serde_json::Value, String> {
                Err("structured fails".into())
            }
        }

        let client: Box<dyn LLMClientTrait> = Box::new(MockLLMClientFallbackParseError);
        let mut extractor = IntentExtractor::new(client, None, None::<Box<dyn SemanticCache>>);

        let result = extractor.extract("Find the Q1 sales report");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Fallback parse failed"));
    }
}

#[cfg(test)]
mod loan_pipeline_tests {
    use crate::capability_router::{ActionType, ResourceType, RouteResult};
    use crate::intent_extractor::IntentExtractor;
    use crate::llm_interface::LLMClientTrait;
    use crate::loan_orchestrator::{
        LoanOrchestratorAgent, MockCreditChecker, MockDocumentValidator, MockRiskAssessor,
    };
    use crate::semantic_cache::SemanticCache;
    use crate::supervisor::WorkflowStatus;
    use serde_json::json;

    struct MockLoanLLMClient {
        action: &'static str,
        resource: &'static str,
        parameters: serde_json::Value,
    }

    impl LLMClientTrait for MockLoanLLMClient {
        fn chat(
            &self,
            _messages: Vec<serde_json::Value>,
            _system_prompt: Option<&str>,
            _response_format: Option<serde_json::Value>,
        ) -> Result<String, String> {
            Err("should not reach chat fallback".into())
        }

        fn structured_chat(
            &self,
            _messages: Vec<serde_json::Value>,
            _schema: serde_json::Value,
            _system_prompt: Option<&str>,
        ) -> Result<serde_json::Value, String> {
            Ok(json!({
                "action": self.action,
                "resource": self.resource,
                "parameters": self.parameters
            }))
        }
    }

    fn build_application_from_params(params: &serde_json::Value) -> String {
        json!({
            "applicant_id": params.get("applicant_id").and_then(|v| v.as_str()).unwrap_or("APP-UNKNOWN"),
            "loan_amount": params.get("loan_amount").and_then(|v| v.as_u64()).unwrap_or(0),
            "loan_type": params.get("loan_type").and_then(|v| v.as_str()).unwrap_or("unknown"),
            "documents": ["tax_return_2024", "pay_stub", "bank_statement"]
        })
        .to_string()
    }

    #[test]
    fn test_extract_loan_intent() {
        let client: Box<dyn LLMClientTrait> = Box::new(MockLoanLLMClient {
            action: "create",
            resource: "loan_application",
            parameters: json!({
                "applicant_id": "APP-E2E-001",
                "loan_amount": 300000,
                "loan_type": "mortgage"
            }),
        });
        let mut extractor = IntentExtractor::new(client, None, None::<Box<dyn SemanticCache>>);
        let intent = extractor.extract("Apply for a mortgage loan").unwrap();
        assert_eq!(intent.action, ActionType::Create);
        assert_eq!(intent.resource, ResourceType::LoanApplication);
    }

    #[test]
    fn test_loan_route_to_orchestrator() {
        let client: Box<dyn LLMClientTrait> = Box::new(MockLoanLLMClient {
            action: "create",
            resource: "loan_application",
            parameters: json!({
                "applicant_id": "APP-E2E-001",
                "loan_amount": 300000,
                "loan_type": "mortgage"
            }),
        });
        let mut extractor = IntentExtractor::new(client, None, None::<Box<dyn SemanticCache>>);
        let route_result = extractor
            .process("I want to apply for a home loan of 300000")
            .unwrap();
        assert!(matches!(route_result, RouteResult::Routed { .. }));
        assert_eq!(route_result.agent_name(), Some("LoanOrchestratorAgent"));
    }

    #[test]
    fn test_loan_full_pipeline_approved() {
        let client: Box<dyn LLMClientTrait> = Box::new(MockLoanLLMClient {
            action: "create",
            resource: "loan_application",
            parameters: json!({
                "applicant_id": "APP-E2E-001",
                "loan_amount": 300000,
                "loan_type": "mortgage"
            }),
        });
        let mut extractor = IntentExtractor::new(client, None, None::<Box<dyn SemanticCache>>);
        let route_result = extractor
            .process("I want to apply for a home loan")
            .unwrap();
        assert_eq!(route_result.agent_name(), Some("LoanOrchestratorAgent"));
        let params = match &route_result {
            RouteResult::Routed { parameters, .. } => parameters.clone(),
            _ => panic!("Expected Routed"),
        };
        let orchestrator = LoanOrchestratorAgent::new(
            MockDocumentValidator::new("doc_validator", true),
            MockCreditChecker::new("credit_checker", 750),
            MockRiskAssessor::new("risk_assessor", "low"),
        );
        let application = build_application_from_params(&params);
        let result = orchestrator.process(&application);
        assert_eq!(result.status, WorkflowStatus::Completed);
        assert_eq!(result.audit_entries.len(), 3);
        let decision = LoanOrchestratorAgent::decision_string(&result);
        assert!(decision.contains("approved"));
    }

    #[test]
    fn test_loan_pipeline_rejected_invalid_docs() {
        let client: Box<dyn LLMClientTrait> = Box::new(MockLoanLLMClient {
            action: "create",
            resource: "loan_application",
            parameters: json!({
                "applicant_id": "APP-E2E-002",
                "loan_amount": 150000,
                "loan_type": "personal"
            }),
        });
        let mut extractor = IntentExtractor::new(client, None, None::<Box<dyn SemanticCache>>);
        let route_result = extractor.process("Apply for a personal loan").unwrap();
        assert_eq!(route_result.agent_name(), Some("LoanOrchestratorAgent"));
        let params = match &route_result {
            RouteResult::Routed { parameters, .. } => parameters.clone(),
            _ => panic!("Expected Routed"),
        };
        let orchestrator = LoanOrchestratorAgent::new(
            MockDocumentValidator::new("doc_validator", false),
            MockCreditChecker::new("credit_checker", 750),
            MockRiskAssessor::new("risk_assessor", "low"),
        );
        let application = build_application_from_params(&params);
        let result = orchestrator.process(&application);
        assert_eq!(result.status, WorkflowStatus::Halted);
        let decision = LoanOrchestratorAgent::decision_string(&result);
        assert!(decision.contains("Invalid Documents"));
    }

    #[test]
    fn test_loan_pipeline_rejected_low_credit() {
        let client: Box<dyn LLMClientTrait> = Box::new(MockLoanLLMClient {
            action: "create",
            resource: "loan_application",
            parameters: json!({
                "applicant_id": "APP-E2E-003",
                "loan_amount": 500000,
                "loan_type": "commercial"
            }),
        });
        let mut extractor = IntentExtractor::new(client, None, None::<Box<dyn SemanticCache>>);
        let route_result = extractor
            .process("I need a commercial loan for my business")
            .unwrap();
        assert_eq!(route_result.agent_name(), Some("LoanOrchestratorAgent"));
        let params = match &route_result {
            RouteResult::Routed { parameters, .. } => parameters.clone(),
            _ => panic!("Expected Routed"),
        };
        let orchestrator = LoanOrchestratorAgent::new(
            MockDocumentValidator::new("doc_validator", true),
            MockCreditChecker::new("credit_checker", 550),
            MockRiskAssessor::new("risk_assessor", "high"),
        );
        let application = build_application_from_params(&params);
        let result = orchestrator.process(&application);
        assert_eq!(result.status, WorkflowStatus::Halted);
        let decision = LoanOrchestratorAgent::decision_string(&result);
        assert!(decision.contains("Low Credit Score"));
    }
}
