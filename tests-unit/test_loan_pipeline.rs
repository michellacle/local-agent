use local_agent::capability_router::{ActionType, ResourceType, RouteResult};
use local_agent::intent_extractor::IntentExtractor;
use local_agent::llm_interface::LLMClientTrait;
use local_agent::loan_orchestrator::{
    LoanOrchestratorAgent, MockCreditChecker, MockDocumentValidator, MockRiskAssessor,
};
use local_agent::semantic_cache::SemanticCache;
use local_agent::supervisor::WorkflowStatus;
use serde_json::json;

/// Configurable mock LLM client for loan intent extraction.
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
