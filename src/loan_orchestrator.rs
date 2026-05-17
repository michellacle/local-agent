use crate::supervisor::{Supervisor, WorkerAgent, WorkflowResult, WorkflowStatus, WorkflowStep};
use serde_json::Value;

/// Mock document validation worker.
pub struct MockDocumentValidator {
    name: String,
    valid: bool,
}

impl MockDocumentValidator {
    pub fn new(name: &str, valid: bool) -> Self {
        Self {
            name: name.into(),
            valid,
        }
    }
}

impl WorkerAgent for MockDocumentValidator {
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(&self, _input: &str) -> Result<String, String> {
        Ok(serde_json::json!({
            "status": if self.valid { "valid" } else { "invalid" },
            "checked": ["income_statement", "id_verification", "bank_statements"]
        })
        .to_string())
    }
}

/// Mock credit check worker.
pub struct MockCreditChecker {
    name: String,
    credit_score: u32,
}

impl MockCreditChecker {
    pub fn new(name: &str, credit_score: u32) -> Self {
        Self {
            name: name.into(),
            credit_score,
        }
    }
}

impl WorkerAgent for MockCreditChecker {
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(&self, _input: &str) -> Result<String, String> {
        Ok(serde_json::json!({
            "credit_score": self.credit_score,
            "bureau": "Equifax",
            "delinquencies": if self.credit_score < 600 { 3 } else { 0 }
        })
        .to_string())
    }
}

/// Mock risk assessment worker.
pub struct MockRiskAssessor {
    name: String,
    risk_level: String,
}

impl MockRiskAssessor {
    pub fn new(name: &str, risk_level: &str) -> Self {
        Self {
            name: name.into(),
            risk_level: risk_level.into(),
        }
    }
}

impl WorkerAgent for MockRiskAssessor {
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(&self, input: &str) -> Result<String, String> {
        let input_json: Value = serde_json::from_str(input).unwrap_or(Value::Null);
        Ok(serde_json::json!({
            "risk_level": self.risk_level,
            "risk_score": match self.risk_level.as_str() {
                "low" => 15,
                "medium" => 45,
                "high" => 82,
                _ => 50,
            },
            "applicant_id": input_json.get("applicant_id").and_then(|v| v.as_str()).unwrap_or("unknown"),
            "decision": if self.risk_level == "low" { "approved" } else { "rejected" }
        })
        .to_string())
    }
}

/// Orchestrates the loan approval workflow using a Supervisor.
///
/// Mirrors the `LoanOrchestratorAgent` from the book:
/// 1. DocumentValidationAgent validates documents
/// 2. CreditCheckAgent checks credit (only if docs are valid)
/// 3. RiskAssessmentAgent assesses risk (only if credit >= 600)
/// 4. Final decision is assembled from the risk assessment
pub struct LoanOrchestratorAgent {
    doc_validator: MockDocumentValidator,
    credit_checker: MockCreditChecker,
    risk_assessor: MockRiskAssessor,
}

impl LoanOrchestratorAgent {
    pub fn new(
        doc_validator: MockDocumentValidator,
        credit_checker: MockCreditChecker,
        risk_assessor: MockRiskAssessor,
    ) -> Self {
        Self {
            doc_validator,
            credit_checker,
            risk_assessor,
        }
    }

    /// Runs the full loan approval workflow on the given application data.
    pub fn process(&self, application_data: &str) -> WorkflowResult {
        let mut supervisor = Supervisor::builder(application_data)
            .add_worker(Box::new(MockDocumentValidator::new(
                "doc_validator",
                self.doc_validator.valid,
            )))
            .unwrap()
            .add_worker(Box::new(MockCreditChecker::new(
                "credit_checker",
                self.credit_checker.credit_score,
            )))
            .unwrap()
            .add_worker(Box::new(MockRiskAssessor::new(
                "risk_assessor",
                &self.risk_assessor.risk_level,
            )))
            .unwrap()
            .add_step(WorkflowStep::new("document_validation", "doc_validator"))
            .add_step(
                WorkflowStep::new("credit_check", "credit_checker").with_condition(|output| {
                    let parsed: Result<serde_json::Value, _> = serde_json::from_str(output);
                    parsed
                        .map(|v| v.get("status") == Some(&serde_json::json!("valid")))
                        .unwrap_or(false)
                }),
            )
            .add_step(
                WorkflowStep::new("risk_assessment", "risk_assessor").with_condition(|output| {
                    let parsed: Result<serde_json::Value, _> = serde_json::from_str(output);
                    parsed
                        .map(|v| v.get("credit_score").and_then(|s| s.as_u64()).unwrap_or(0) >= 600)
                        .unwrap_or(false)
                }),
            )
            .build()
            .unwrap();

        supervisor.execute()
    }

    /// Returns a human-readable decision string from a workflow result.
    pub fn decision_string(result: &WorkflowResult) -> String {
        match result.status {
            WorkflowStatus::Completed => {
                let final_json: serde_json::Value =
                    serde_json::from_str(&result.final_output).unwrap_or_default();
                let decision = final_json
                    .get("decision")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                format!(
                    "Loan {}: risk_level={}, risk_score={}",
                    decision,
                    final_json
                        .get("risk_level")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown"),
                    final_json
                        .get("risk_score")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0),
                )
            }
            WorkflowStatus::Halted => {
                let last_entry = result.audit_entries.last();
                if let Some(entry) = last_entry {
                    if entry.output.contains("Condition not met") {
                        let reason = if entry.step_name == "credit_check" {
                            "Invalid Documents"
                        } else if entry.step_name == "risk_assessment" {
                            "Low Credit Score"
                        } else {
                            "Unknown"
                        };
                        format!("Application Rejected: {}", reason)
                    } else {
                        "Application Rejected: Unknown halt reason".into()
                    }
                } else {
                    "Application Rejected: No audit entries".into()
                }
            }
            WorkflowStatus::Failed => "Application Rejected: Critical workflow failure".into(),
            WorkflowStatus::Running => "Application: Still processing".into(),
        }
    }
}
