use local_agent::loan_orchestrator::{
    LoanOrchestratorAgent, MockCreditChecker, MockDocumentValidator, MockRiskAssessor,
};
use local_agent::supervisor::WorkflowStatus;

/// Helper to build a loan orchestrator with all valid data.
fn build_valid_loan_orchestrator() -> LoanOrchestratorAgent {
    LoanOrchestratorAgent::new(
        MockDocumentValidator::new("doc_validator", true),
        MockCreditChecker::new("credit_checker", 750),
        MockRiskAssessor::new("risk_assessor", "low"),
    )
}

/// Helper to build a loan orchestrator with invalid documents.
fn build_invalid_doc_loan_orchestrator() -> LoanOrchestratorAgent {
    LoanOrchestratorAgent::new(
        MockDocumentValidator::new("doc_validator", false),
        MockCreditChecker::new("credit_checker", 750),
        MockRiskAssessor::new("risk_assessor", "low"),
    )
}

/// Helper to build a loan orchestrator with low credit score.
fn build_low_credit_loan_orchestrator() -> LoanOrchestratorAgent {
    LoanOrchestratorAgent::new(
        MockDocumentValidator::new("doc_validator", true),
        MockCreditChecker::new("credit_checker", 550),
        MockRiskAssessor::new("risk_assessor", "high"),
    )
}

#[test]
fn test_loan_approval_full_workflow() {
    let orchestrator = build_valid_loan_orchestrator();
    let application = serde_json::json!({
        "applicant_id": "APP-12345",
        "loan_amount": 250000,
        "loan_type": "mortgage",
        "documents": ["tax_return_2024", "pay_stub", "bank_statement"]
    })
    .to_string();

    let result = orchestrator.process(&application);

    // Should complete all 3 steps
    assert_eq!(result.status, WorkflowStatus::Completed);
    assert_eq!(result.audit_entries.len(), 3);

    // Verify step order
    assert_eq!(result.audit_entries[0].step_name, "document_validation");
    assert_eq!(result.audit_entries[1].step_name, "credit_check");
    assert_eq!(result.audit_entries[2].step_name, "risk_assessment");

    // All steps should succeed
    assert!(result.audit_entries[0].success);
    assert!(result.audit_entries[1].success);
    assert!(result.audit_entries[2].success);

    // Verify decision string
    let decision = LoanOrchestratorAgent::decision_string(&result);
    assert!(decision.contains("approved"));
    assert!(decision.contains("low"));
}

#[test]
fn test_loan_rejected_invalid_documents() {
    let orchestrator = build_invalid_doc_loan_orchestrator();
    let application = serde_json::json!({
        "applicant_id": "APP-67890",
        "loan_amount": 150000,
        "loan_type": "personal",
        "documents": ["missing_documents"]
    })
    .to_string();

    let result = orchestrator.process(&application);

    // Should halt at credit_check step
    assert_eq!(result.status, WorkflowStatus::Halted);
    assert_eq!(result.audit_entries.len(), 2);

    // First step succeeds, second halts
    assert!(result.audit_entries[0].success);
    assert!(!result.audit_entries[1].success);
    assert_eq!(result.audit_entries[1].step_name, "credit_check");
    assert!(result.audit_entries[1].output.contains("Condition not met"));

    // Verify decision string
    let decision = LoanOrchestratorAgent::decision_string(&result);
    assert!(decision.contains("Invalid Documents"));
}

#[test]
fn test_loan_rejected_low_credit_score() {
    let orchestrator = build_low_credit_loan_orchestrator();
    let application = serde_json::json!({
        "applicant_id": "APP-11111",
        "loan_amount": 500000,
        "loan_type": "commercial",
        "documents": ["full_documentation"]
    })
    .to_string();

    let result = orchestrator.process(&application);

    // Should halt at risk_assessment step
    assert_eq!(result.status, WorkflowStatus::Halted);
    assert_eq!(result.audit_entries.len(), 3);

    // First two steps succeed, third halts
    assert!(result.audit_entries[0].success);
    assert!(result.audit_entries[1].success);
    assert!(!result.audit_entries[2].success);
    assert_eq!(result.audit_entries[2].step_name, "risk_assessment");

    // Verify decision string
    let decision = LoanOrchestratorAgent::decision_string(&result);
    assert!(decision.contains("Low Credit Score"));
}

#[test]
fn test_loan_workflow_audit_trail() {
    let orchestrator = build_valid_loan_orchestrator();
    let application = serde_json::json!({
        "applicant_id": "APP-AUDIT",
        "loan_amount": 100000
    })
    .to_string();

    let result = orchestrator.process(&application);

    // Verify audit trail contains all required fields
    for entry in &result.audit_entries {
        assert!(!entry.step_name.is_empty());
        assert!(!entry.worker_name.is_empty());
        assert!(!entry.input.is_empty());
        assert!(!entry.output.is_empty());
    }

    // Verify step outputs are structured JSON
    for output in result.step_outputs.values() {
        let parsed: serde_json::Value = serde_json::from_str(output).unwrap();
        assert!(parsed.is_object());
    }
}

#[test]
fn test_loan_workflow_sequential_delegation() {
    let orchestrator = build_valid_loan_orchestrator();
    let application = serde_json::json!({
        "applicant_id": "APP-SEQ",
        "loan_amount": 300000
    })
    .to_string();

    let result = orchestrator.process(&application);

    // Verify that each step received the previous step's output as input
    // First step gets the application data
    assert!(result.audit_entries[0].input.contains("APP-SEQ"));

    // Second step gets the document validation output
    assert!(result.audit_entries[1].input.contains("valid"));

    // Third step gets the credit check output
    assert!(result.audit_entries[2].input.contains("credit_score"));
}
