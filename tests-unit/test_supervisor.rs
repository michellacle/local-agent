use local_agent::supervisor::{
    Supervisor, WorkerAgent, WorkflowResult, WorkflowStatus, WorkflowStep,
};

// --- Test Workers ---

/// Echoes the input back as output.
struct EchoWorker {
    name: String,
}
impl WorkerAgent for EchoWorker {
    fn name(&self) -> &str {
        &self.name
    }
    fn execute(&self, input: &str) -> Result<String, String> {
        Ok(format!("echoed: {input}"))
    }
}

/// Prepends a prefix to the input.
struct PrefixWorker {
    name: String,
    prefix: String,
}
impl WorkerAgent for PrefixWorker {
    fn name(&self) -> &str {
        &self.name
    }
    fn execute(&self, input: &str) -> Result<String, String> {
        Ok(format!("{}{}", self.prefix, input))
    }
}

/// Always returns an error.
struct FailWorker {
    name: String,
    error_msg: String,
}
impl WorkerAgent for FailWorker {
    fn name(&self) -> &str {
        &self.name
    }
    fn execute(&self, _input: &str) -> Result<String, String> {
        Err(self.error_msg.clone())
    }
}

/// Counts how many times it's been called (via a shared counter).
struct CountWorker {
    name: String,
    count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}
impl WorkerAgent for CountWorker {
    fn name(&self) -> &str {
        &self.name
    }
    fn execute(&self, input: &str) -> Result<String, String> {
        let n = self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        Ok(format!("{input} (call #{n})"))
    }
}

// --- TC1: Supervisor creates with registered workers ---

#[test]
fn test_supervisor_creates_with_workers() {
    let result = Supervisor::builder("process loan")
        .add_worker(Box::new(EchoWorker {
            name: "validator".into(),
        }))
        .unwrap()
        .add_worker(Box::new(EchoWorker {
            name: "checker".into(),
        }))
        .unwrap()
        .build();
    assert!(result.is_ok());
}

// --- TC2: Supervisor rejects empty team ---

#[test]
fn test_supervisor_rejects_no_workers() {
    let result = Supervisor::builder("goal").build();
    assert!(result.is_err());
    assert!(result.err().unwrap().contains("at least one worker"));
}

// --- TC3: Supervisor rejects duplicate worker names ---

#[test]
fn test_supervisor_rejects_duplicate_workers() {
    let result = Supervisor::builder("goal")
        .add_worker(Box::new(EchoWorker {
            name: "same".into(),
        }))
        .unwrap()
        .add_worker(Box::new(EchoWorker {
            name: "same".into(),
        }));
    assert!(result.is_err());
    assert!(result.err().unwrap().contains("Duplicate"));
}

// --- TC4: 2-step sequential delegation, output of step 1 -> input of step 2 ---

#[test]
fn test_supervisor_sequential_two_steps() {
    let mut sup = Supervisor::builder("initial")
        .add_worker(Box::new(EchoWorker {
            name: "step1".into(),
        }))
        .unwrap()
        .add_worker(Box::new(PrefixWorker {
            name: "step2".into(),
            prefix: "[PREFIX]".into(),
        }))
        .unwrap()
        .add_step(WorkflowStep::new("validate", "step1"))
        .add_step(WorkflowStep::new("prefix", "step2"))
        .build()
        .unwrap();

    let result = sup.execute();

    assert_eq!(result.status, WorkflowStatus::Completed);
    assert_eq!(result.step_outputs["validate"], "echoed: initial");
    // step2 receives step1's output as input
    assert_eq!(result.step_outputs["prefix"], "[PREFIX]echoed: initial");
}

// --- TC5: 3+ step chain, outputs captured in order ---

#[test]
fn test_supervisor_sequential_three_steps() {
    let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut sup = Supervisor::builder("start")
        .add_worker(Box::new(EchoWorker { name: "w1".into() }))
        .unwrap()
        .add_worker(Box::new(CountWorker {
            name: "w2".into(),
            count: count.clone(),
        }))
        .unwrap()
        .add_worker(Box::new(EchoWorker { name: "w3".into() }))
        .unwrap()
        .add_step(WorkflowStep::new("s1", "w1"))
        .add_step(WorkflowStep::new("s2", "w2"))
        .add_step(WorkflowStep::new("s3", "w3"))
        .build()
        .unwrap();

    let result = sup.execute();

    assert_eq!(result.status, WorkflowStatus::Completed);
    assert_eq!(result.audit_entries.len(), 3);
    assert_eq!(result.audit_entries[0].step_name, "s1");
    assert_eq!(result.audit_entries[1].step_name, "s2");
    assert_eq!(result.audit_entries[2].step_name, "s3");
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 1);
}

// --- TC6: Final result is assembled from last step output ---

#[test]
fn test_supervisor_final_result() {
    let mut sup = Supervisor::builder("goal")
        .add_worker(Box::new(PrefixWorker {
            name: "worker".into(),
            prefix: "DONE:".into(),
        }))
        .unwrap()
        .add_step(WorkflowStep::new("final_step", "worker"))
        .build()
        .unwrap();

    let result = sup.execute();

    assert_eq!(result.status, WorkflowStatus::Completed);
    assert_eq!(result.final_output, "DONE:goal");
}

// --- TC7: Conditional routing - condition met continues ---

#[test]
fn test_supervisor_condition_met_continues() {
    let mut sup = Supervisor::builder("valid data")
        .add_worker(Box::new(EchoWorker {
            name: "validator".into(),
        }))
        .unwrap()
        .add_worker(Box::new(EchoWorker {
            name: "processor".into(),
        }))
        .unwrap()
        .add_step(WorkflowStep::new("validate", "validator"))
        .add_step(
            WorkflowStep::new("process", "processor")
                .with_condition(|output| output.contains("echoed")),
        )
        .build()
        .unwrap();

    let result = sup.execute();

    assert_eq!(result.status, WorkflowStatus::Completed);
    assert_eq!(result.audit_entries.len(), 2);
}

// --- TC8: Conditional routing - condition not met halts ---

#[test]
fn test_supervisor_condition_not_met_halts() {
    let mut sup = Supervisor::builder("initial")
        .add_worker(Box::new(EchoWorker {
            name: "validator".into(),
        }))
        .unwrap()
        .add_worker(Box::new(EchoWorker {
            name: "processor".into(),
        }))
        .unwrap()
        .add_step(WorkflowStep::new("validate", "validator"))
        .add_step(
            WorkflowStep::new("process", "processor").with_condition(|_output| {
                false // Always fails
            }),
        )
        .build()
        .unwrap();

    let result = sup.execute();

    assert_eq!(result.status, WorkflowStatus::Halted);
    assert_eq!(result.audit_entries.len(), 2);
    assert!(result.audit_entries[1].output.contains("Condition not met"));
}

// --- TC9: Terminal condition halts early ---

#[test]
fn test_supervisor_terminal_halt() {
    let mut sup = Supervisor::builder("initial")
        .add_worker(Box::new(EchoWorker { name: "w1".into() }))
        .unwrap()
        .add_worker(Box::new(EchoWorker { name: "w2".into() }))
        .unwrap()
        .add_worker(Box::new(EchoWorker { name: "w3".into() }))
        .unwrap()
        .add_step(WorkflowStep::new("s1", "w1"))
        .add_step(
            WorkflowStep::new("s2", "w2").with_condition(|_| false), // Halt here
        )
        .add_step(WorkflowStep::new("s3", "w3"))
        .build()
        .unwrap();

    let result = sup.execute();

    assert_eq!(result.status, WorkflowStatus::Halted);
    // s3 should NOT have executed
    assert!(!result.step_outputs.contains_key("s3"));
}

// --- TC10: Retry on failure (up to N attempts) ---

#[test]
fn test_supervisor_retry_failed_worker() {
    // This test verifies that a non-critical step that fails
    // allows the workflow to continue. Full retry logic will be
    // added in Phase 3 with configurable max_retries.
    let mut sup = Supervisor::builder("goal")
        .add_worker(Box::new(FailWorker {
            name: "flaky".into(),
            error_msg: "temporary failure".into(),
        }))
        .unwrap()
        .add_worker(Box::new(EchoWorker {
            name: "recovery".into(),
        }))
        .unwrap()
        .add_step(WorkflowStep::new("flaky_step", "flaky").non_critical())
        .add_step(WorkflowStep::new("recover", "recovery"))
        .build()
        .unwrap();

    let result = sup.execute();

    // Workflow should NOT have failed (non-critical step)
    assert_ne!(result.status, WorkflowStatus::Failed);
    assert_eq!(result.status, WorkflowStatus::Completed);
    assert!(!result.audit_entries[0].success);
    assert!(result.audit_entries[1].success);
}

// --- TC12: Non-critical failure returns partial results ---

#[test]
fn test_supervisor_non_critical_failure_partial_results() {
    let mut sup = Supervisor::builder("goal")
        .add_worker(Box::new(EchoWorker {
            name: "good".into(),
        }))
        .unwrap()
        .add_worker(Box::new(FailWorker {
            name: "bad".into(),
            error_msg: "oops".into(),
        }))
        .unwrap()
        .add_step(WorkflowStep::new("good_step", "good"))
        .add_step(WorkflowStep::new("bad_step", "bad").non_critical())
        .build()
        .unwrap();

    let result = sup.execute();

    assert_eq!(result.status, WorkflowStatus::Completed);
    assert!(result.step_outputs.contains_key("good_step"));
    assert!(!result.step_outputs.contains_key("bad_step"));
}

// --- TC13: Critical step failure aborts immediately ---

#[test]
fn test_supervisor_critical_failure_aborts() {
    let mut sup = Supervisor::builder("goal")
        .add_worker(Box::new(EchoWorker {
            name: "good".into(),
        }))
        .unwrap()
        .add_worker(Box::new(FailWorker {
            name: "bad".into(),
            error_msg: "critical error".into(),
        }))
        .unwrap()
        .add_worker(Box::new(EchoWorker {
            name: "never_runs".into(),
        }))
        .unwrap()
        .add_step(WorkflowStep::new("good_step", "good"))
        .add_step(WorkflowStep::new("bad_step", "bad")) // critical by default
        .add_step(WorkflowStep::new("final_step", "never_runs"))
        .build()
        .unwrap();

    let result = sup.execute();

    assert_eq!(result.status, WorkflowStatus::Failed);
    assert!(!result.step_outputs.contains_key("final_step"));
    assert_eq!(result.audit_entries.len(), 2);
}

// --- TC17: Structured output schema validation ---

#[test]
fn test_supervisor_worker_output_is_structured() {
    struct StructuredWorker;
    impl WorkerAgent for StructuredWorker {
        fn name(&self) -> &str {
            "structured"
        }
        fn execute(&self, input: &str) -> Result<String, String> {
            // Returns valid JSON
            Ok(serde_json::json!({"status": "ok", "data": input}).to_string())
        }
    }

    let mut sup = Supervisor::builder("query")
        .add_worker(Box::new(StructuredWorker))
        .unwrap()
        .add_step(WorkflowStep::new("analyze", "structured"))
        .build()
        .unwrap();

    let result = sup.execute();

    let output = &result.step_outputs["analyze"];
    let parsed: serde_json::Value = serde_json::from_str(output).unwrap();
    assert_eq!(parsed["status"], "ok");
    assert_eq!(parsed["data"], "query");
}

// --- TC18: Structured context passed between workers ---

#[test]
fn test_supervisor_structured_context_passing() {
    struct JsonPassWorker {
        name: String,
    }
    impl WorkerAgent for JsonPassWorker {
        fn name(&self) -> &str {
            &self.name
        }
        fn execute(&self, input: &str) -> Result<String, String> {
            let mut obj: serde_json::Value =
                serde_json::from_str(input).unwrap_or(serde_json::json!({"steps": []}));
            obj["steps"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!(self.name));
            Ok(obj.to_string())
        }
    }

    let mut sup = Supervisor::builder(serde_json::json!({"steps": []}).to_string().as_str())
        .add_worker(Box::new(JsonPassWorker {
            name: "first".into(),
        }))
        .unwrap()
        .add_worker(Box::new(JsonPassWorker {
            name: "second".into(),
        }))
        .unwrap()
        .add_step(WorkflowStep::new("s1", "first"))
        .add_step(WorkflowStep::new("s2", "second"))
        .build()
        .unwrap();

    let result = sup.execute();

    let final_json: serde_json::Value = serde_json::from_str(&result.final_output).unwrap();
    let steps = final_json["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0], "first");
    assert_eq!(steps[1], "second");
}

// --- TC19: Audit trail records step details ---

#[test]
fn test_supervisor_audit_trail_records_details() {
    let mut sup = Supervisor::builder("my goal")
        .add_worker(Box::new(EchoWorker {
            name: "worker".into(),
        }))
        .unwrap()
        .add_step(WorkflowStep::new("my_step", "worker"))
        .build()
        .unwrap();

    let result = sup.execute();

    assert_eq!(result.audit_entries.len(), 1);
    let entry = &result.audit_entries[0];
    assert_eq!(entry.step_name, "my_step");
    assert_eq!(entry.worker_name, "worker");
    assert_eq!(entry.input, "my goal");
    assert!(entry.success);
}

// --- TC20: Audit trail returned as part of result ---

#[test]
fn test_supervisor_audit_in_result() {
    let mut sup = Supervisor::builder("goal")
        .add_worker(Box::new(EchoWorker { name: "w".into() }))
        .unwrap()
        .add_step(WorkflowStep::new("s1", "w"))
        .add_step(WorkflowStep::new("s2", "w"))
        .build()
        .unwrap();

    let result = sup.execute();

    assert_eq!(result.audit_entries.len(), 2);
    assert_eq!(result.audit_entries[0].step_name, "s1");
    assert_eq!(result.audit_entries[1].step_name, "s2");
}

// --- TC22: Supervisor contains no domain logic ---

#[test]
fn test_supervisor_no_domain_logic() {
    // The supervisor only routes and tracks state.
    // All actual work is done by the worker.
    struct DomainWorker;
    impl WorkerAgent for DomainWorker {
        fn name(&self) -> &str {
            "domain"
        }
        fn execute(&self, input: &str) -> Result<String, String> {
            // Domain-specific logic lives here, not in supervisor
            Ok(format!("processed-domain-{}", input))
        }
    }

    let mut sup = Supervisor::builder("data")
        .add_worker(Box::new(DomainWorker))
        .unwrap()
        .add_step(WorkflowStep::new("process", "domain"))
        .build()
        .unwrap();

    let result = sup.execute();
    // Supervisor just passed input/output, no domain logic of its own
    assert_eq!(result.step_outputs["process"], "processed-domain-data");
}

// --- TC23: Worker agents independently testable ---

#[test]
fn test_worker_independent_of_supervisor() {
    let worker = EchoWorker {
        name: "standalone".into(),
    };
    let output = worker.execute("hello").unwrap();
    assert_eq!(output, "echoed: hello");
    assert_eq!(worker.name(), "standalone");
}

// --- Additional: Builder validates worker references ---

#[test]
fn test_builder_rejects_unregistered_worker() {
    let result = Supervisor::builder("goal")
        .add_worker(Box::new(EchoWorker {
            name: "real".into(),
        }))
        .unwrap()
        .add_step(WorkflowStep::new("step", "nonexistent"))
        .build();
    assert!(result.is_err());
    assert!(result.err().unwrap().contains("unregistered"));
}

// --- Additional: Empty workflow completes immediately ---

#[test]
fn test_supervisor_empty_steps_completes() {
    let mut sup = Supervisor::builder("goal")
        .add_worker(Box::new(EchoWorker { name: "w".into() }))
        .unwrap()
        .build()
        .unwrap();

    let result = sup.execute();

    assert_eq!(result.status, WorkflowStatus::Completed);
    assert!(result.step_outputs.is_empty());
    assert!(result.audit_entries.is_empty());
}

// --- Additional: First step receives goal as input ---

#[test]
fn test_first_step_receives_goal() {
    let mut sup = Supervisor::builder("my goal text")
        .add_worker(Box::new(EchoWorker { name: "w".into() }))
        .unwrap()
        .add_step(WorkflowStep::new("s1", "w"))
        .build()
        .unwrap();

    let result = sup.execute();

    assert_eq!(result.audit_entries[0].input, "my goal text");
}

// --- Additional: WorkflowResult is serializable ---

#[test]
fn test_workflow_result_serialization() {
    let mut sup = Supervisor::builder("goal")
        .add_worker(Box::new(EchoWorker { name: "w".into() }))
        .unwrap()
        .add_step(WorkflowStep::new("s1", "w"))
        .build()
        .unwrap();

    let result = sup.execute();
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("Completed"));
    assert!(json.contains("s1"));

    let deserialized: WorkflowResult = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.status, WorkflowStatus::Completed);
}
