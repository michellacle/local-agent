use std::sync::{Mutex, OnceLock};

use local_agent::intent_extractor::IntentExtractor;
use local_agent::llm_interface::LLMConfig;
use local_agent::loan_orchestrator::{
    LoanOrchestratorAgent, MockCreditChecker, MockDocumentValidator, MockRiskAssessor,
};
use local_agent::semantic_cache::SemanticCache;
use local_agent::supervisor::WorkflowStatus;

static OLLAMA_CHECK: OnceLock<()> = OnceLock::new();
static LIVE_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn lock_live_test() -> std::sync::MutexGuard<'static, ()> {
    LIVE_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn ensure_ollama() {
    OLLAMA_CHECK.get_or_init(|| {
        let resp = ureq::get("http://localhost:11434/api/tags")
            .timeout(std::time::Duration::from_secs(5))
            .call();
        match resp {
            Ok(r) => {
                let _ = r.into_string();
            }
            Err(e) => {
                panic!("Ollama is not running or unreachable: {e}\nStart Ollama and ensure model qwen3.5:2b is pulled.");
            }
        }
    });
}

fn make_extractor() -> IntentExtractor {
    let mut config = LLMConfig::ollama("localhost", "qwen3.5:2b", true);
    config.max_tokens = 256;
    config.timeout = 120;
    let client: Box<dyn local_agent::llm_interface::LLMClientTrait> =
        Box::new(local_agent::llm_interface::LLMClient::new(Some(config)));
    IntentExtractor::new(client, None, None::<Box<dyn SemanticCache>>)
}

fn build_application_from_params(params: &serde_json::Value) -> String {
    serde_json::json!({
        "applicant_id": params.get("applicant_id").and_then(|v| v.as_str()).unwrap_or("APP-UNKNOWN"),
        "loan_amount": params.get("loan_amount").and_then(|v| v.as_u64()).unwrap_or(0),
        "loan_type": params.get("loan_type").and_then(|v| v.as_str()).unwrap_or("unknown"),
        "documents": ["tax_return_2024", "pay_stub", "bank_statement"]
    })
    .to_string()
}

/// Real integration test: Ollama extracts loan intent -> routes to LoanOrchestratorAgent -> approved workflow.
#[test]
fn test_loan_intent_extraction_and_approval() {
    let _guard = lock_live_test();
    ensure_ollama();
    let mut extractor = make_extractor();

    let route_result = extractor
        .process("I want to apply for a mortgage loan of 300000 for applicant APP-INTEGRATION-001")
        .unwrap();

    assert!(route_result.is_routed());
    assert_eq!(route_result.agent_name(), Some("LoanOrchestratorAgent"));

    let params = match &route_result {
        local_agent::RouteResult::Routed { parameters, .. } => parameters.clone(),
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
    assert!(
        decision.contains("approved"),
        "Expected approval, got: {}",
        decision
    );
}

/// Real integration test: Ollama extracts loan intent -> routes to LoanOrchestratorAgent -> rejected on invalid docs.
#[test]
fn test_loan_intent_extraction_rejected_invalid_docs() {
    let _guard = lock_live_test();
    ensure_ollama();
    let mut extractor = make_extractor();

    let route_result = extractor
        .process("Apply for a personal loan of 150000 for applicant APP-INTEGRATION-002")
        .unwrap();

    assert!(route_result.is_routed());
    assert_eq!(route_result.agent_name(), Some("LoanOrchestratorAgent"));

    let params = match &route_result {
        local_agent::RouteResult::Routed { parameters, .. } => parameters.clone(),
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
    assert!(
        decision.contains("Invalid Documents"),
        "Expected rejection for invalid docs, got: {}",
        decision
    );
}

/// Real integration test: Ollama extracts loan intent -> routes to LoanOrchestratorAgent -> rejected on low credit.
#[test]
fn test_loan_intent_extraction_rejected_low_credit() {
    let _guard = lock_live_test();
    ensure_ollama();
    let mut extractor = make_extractor();

    let route_result = extractor
        .process(
            "I need a commercial loan of 500000 for my business, applicant APP-INTEGRATION-003",
        )
        .unwrap();

    assert!(route_result.is_routed());
    assert_eq!(route_result.agent_name(), Some("LoanOrchestratorAgent"));

    let params = match &route_result {
        local_agent::RouteResult::Routed { parameters, .. } => parameters.clone(),
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
    assert!(
        decision.contains("Low Credit Score"),
        "Expected rejection for low credit, got: {}",
        decision
    );
}
