use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Trait for specialized worker agents that execute a single domain task.
pub trait WorkerAgent: Send + Sync {
    /// Returns the unique name of this worker agent.
    fn name(&self) -> &str;

    /// Executes the worker's task on the given input string.
    fn execute(&self, input: &str) -> Result<String, String>;
}

/// The current status of a workflow execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorkflowStatus {
    /// Workflow is currently running.
    Running,
    /// Workflow completed successfully.
    Completed,
    /// Workflow failed due to a critical error.
    Failed,
    /// Workflow was halted early by a condition.
    Halted,
}

/// A single step in a supervisor workflow.
pub struct WorkflowStep {
    /// Human-readable name for this step.
    pub name: String,
    /// Name of the worker agent to execute this step.
    pub worker_name: String,
    /// Optional condition to evaluate on the previous step's output.
    /// If present and evaluates to false, the workflow halts.
    #[allow(clippy::type_complexity)]
    pub condition: Option<Box<dyn Fn(&str) -> bool>>,
    /// If true, a failure in this step immediately fails the workflow.
    pub critical: bool,
}

impl WorkflowStep {
    /// Creates a new workflow step with the given name and target worker.
    pub fn new(name: impl Into<String>, worker_name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            worker_name: worker_name.into(),
            condition: None,
            critical: true,
        }
    }

    /// Sets an optional condition that must be true on the previous step's output.
    pub fn with_condition(mut self, cond: impl Fn(&str) -> bool + 'static) -> Self {
        self.condition = Some(Box::new(cond));
        self
    }

    /// Marks this step as non-critical (failure won't abort the workflow).
    pub fn non_critical(mut self) -> Self {
        self.critical = false;
        self
    }
}

/// An entry in the workflow audit log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// The step that produced this entry.
    pub step_name: String,
    /// The worker agent that executed the step.
    pub worker_name: String,
    /// The input passed to the worker.
    pub input: String,
    /// The output returned by the worker (or error message).
    pub output: String,
    /// Whether the step succeeded.
    pub success: bool,
}

/// The result of a complete workflow execution.
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowResult {
    /// The output of the final executed step.
    pub final_output: String,
    /// Map of step name -> step output for all executed steps.
    pub step_outputs: HashMap<String, String>,
    /// The final status of the workflow.
    pub status: WorkflowStatus,
    /// Ordered audit log of all executed steps.
    pub audit_entries: Vec<AuditEntry>,
}

/// Central orchestrator that delegates work to registered worker agents.
pub struct Supervisor {
    /// Registered worker agents by name.
    workers: HashMap<String, Box<dyn WorkerAgent>>,
    /// Ordered list of workflow steps.
    steps: Vec<WorkflowStep>,
    /// The initial input/goal for the workflow.
    goal: String,
    /// Index of the next step to execute.
    current_step: usize,
    /// Accumulated step outputs.
    step_outputs: HashMap<String, String>,
    /// Ordered audit log.
    audit_entries: Vec<AuditEntry>,
    /// Current workflow status.
    status: WorkflowStatus,
}

/// Builder for constructing a `Supervisor` with validation.
pub struct SupervisorBuilder {
    workers: HashMap<String, Box<dyn WorkerAgent>>,
    steps: Vec<WorkflowStep>,
    goal: String,
}

impl SupervisorBuilder {
    /// Creates a new builder with the initial goal.
    pub fn new(goal: impl Into<String>) -> Self {
        Self {
            workers: HashMap::new(),
            steps: Vec::new(),
            goal: goal.into(),
        }
    }

    /// Registers a worker agent. Returns an error if the name is already taken.
    pub fn add_worker(mut self, worker: Box<dyn WorkerAgent>) -> Result<Self, String> {
        let name = worker.name().to_string();
        if self.workers.contains_key(&name) {
            return Err(format!("Duplicate worker name: {name}"));
        }
        self.workers.insert(name, worker);
        Ok(self)
    }

    /// Adds a workflow step.
    pub fn add_step(mut self, step: WorkflowStep) -> Self {
        self.steps.push(step);
        self
    }

    /// Builds the supervisor, validating the configuration.
    pub fn build(self) -> Result<Supervisor, String> {
        if self.workers.is_empty() {
            return Err("Supervisor requires at least one worker agent".into());
        }

        for step in &self.steps {
            if !self.workers.contains_key(&step.worker_name) {
                return Err(format!(
                    "Step '{}' references unregistered worker '{}'",
                    step.name, step.worker_name
                ));
            }
        }

        Ok(Supervisor {
            workers: self.workers,
            steps: self.steps,
            goal: self.goal,
            current_step: 0,
            step_outputs: HashMap::new(),
            audit_entries: Vec::new(),
            status: WorkflowStatus::Running,
        })
    }
}

impl Supervisor {
    /// Creates a new supervisor using the builder pattern.
    ///
    /// # Example
    /// ```
    /// use local_agent::supervisor::{Supervisor, WorkflowStep, WorkerAgent};
    ///
    /// struct EchoAgent;
    /// impl WorkerAgent for EchoAgent {
    ///     fn name(&self) -> &str { "echo" }
    ///     fn execute(&self, input: &str) -> Result<String, String> { Ok(input.to_string()) }
    /// }
    ///
    /// let sup = Supervisor::builder("hello")
    ///     .add_worker(Box::new(EchoAgent))
    ///     .unwrap()
    ///     .add_step(WorkflowStep::new("step1", "echo"))
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn builder(goal: impl Into<String>) -> SupervisorBuilder {
        SupervisorBuilder::new(goal)
    }

    /// Executes the workflow from the current step to completion.
    pub fn execute(&mut self) -> WorkflowResult {
        while self.current_step < self.steps.len() {
            let step = &self.steps[self.current_step];

            // Determine input: goal for first step, previous output otherwise
            let input = if self.current_step == 0 {
                self.goal.clone()
            } else {
                self.step_outputs
                    .get(&self.steps[self.current_step - 1].name)
                    .cloned()
                    .unwrap_or_default()
            };

            // Evaluate condition on previous step's output (if any)
            if let Some(ref cond) = step.condition {
                let prev_output = if self.current_step > 0 {
                    self.step_outputs
                        .get(&self.steps[self.current_step - 1].name)
                        .map(|s| s.as_str())
                        .unwrap_or("")
                } else {
                    ""
                };
                if !cond(prev_output) {
                    self.status = WorkflowStatus::Halted;
                    self.audit_entries.push(AuditEntry {
                        step_name: step.name.clone(),
                        worker_name: step.worker_name.clone(),
                        input: input.clone(),
                        output: "Condition not met, workflow halted".into(),
                        success: false,
                    });
                    break;
                }
            }

            // Get the worker and execute
            let worker = self
                .workers
                .get(&step.worker_name)
                .expect("Worker not found (validation should have caught this)");

            let result = worker.execute(&input);

            let entry = match result {
                Ok(output) => {
                    self.step_outputs.insert(step.name.clone(), output.clone());
                    AuditEntry {
                        step_name: step.name.clone(),
                        worker_name: step.worker_name.clone(),
                        input,
                        output,
                        success: true,
                    }
                }
                Err(err) => {
                    let output = format!("Error: {err}");
                    AuditEntry {
                        step_name: step.name.clone(),
                        worker_name: step.worker_name.clone(),
                        input,
                        output: output.clone(),
                        success: false,
                    }
                }
            };

            self.audit_entries.push(entry);

            // Check if step failed
            if !self.audit_entries.last().unwrap().success && step.critical {
                self.status = WorkflowStatus::Failed;
                break;
            }

            self.current_step += 1;
        }

        if self.status == WorkflowStatus::Running && self.current_step >= self.steps.len() {
            self.status = WorkflowStatus::Completed;
        }

        let final_output = self
            .audit_entries
            .iter()
            .rev()
            .find(|e| e.success)
            .map(|e| e.output.clone())
            .unwrap_or_default();

        WorkflowResult {
            final_output,
            step_outputs: self.step_outputs.clone(),
            status: self.status.clone(),
            audit_entries: self.audit_entries.clone(),
        }
    }

    /// Returns the current status of the workflow.
    pub fn status(&self) -> &WorkflowStatus {
        &self.status
    }

    /// Returns the audit entries recorded so far.
    pub fn audit_entries(&self) -> &[AuditEntry] {
        &self.audit_entries
    }

    /// Returns the outputs collected so far.
    pub fn step_outputs(&self) -> &HashMap<String, String> {
        &self.step_outputs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_supervisor_rejects_no_workers() {
        let result = Supervisor::builder("goal").build();
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("at least one worker"));
    }

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
        assert_eq!(result.step_outputs["prefix"], "[PREFIX]echoed: initial");
    }

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
            .add_step(WorkflowStep::new("process", "processor").with_condition(|_output| false))
            .build()
            .unwrap();
        let result = sup.execute();
        assert_eq!(result.status, WorkflowStatus::Halted);
        assert_eq!(result.audit_entries.len(), 2);
        assert!(result.audit_entries[1].output.contains("Condition not met"));
    }

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
            .add_step(WorkflowStep::new("s2", "w2").with_condition(|_| false))
            .add_step(WorkflowStep::new("s3", "w3"))
            .build()
            .unwrap();
        let result = sup.execute();
        assert_eq!(result.status, WorkflowStatus::Halted);
        assert!(!result.step_outputs.contains_key("s3"));
    }

    #[test]
    fn test_supervisor_retry_failed_worker() {
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
        assert_ne!(result.status, WorkflowStatus::Failed);
        assert_eq!(result.status, WorkflowStatus::Completed);
        assert!(!result.audit_entries[0].success);
        assert!(result.audit_entries[1].success);
    }

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
            .add_step(WorkflowStep::new("bad_step", "bad"))
            .add_step(WorkflowStep::new("final_step", "never_runs"))
            .build()
            .unwrap();
        let result = sup.execute();
        assert_eq!(result.status, WorkflowStatus::Failed);
        assert!(!result.step_outputs.contains_key("final_step"));
        assert_eq!(result.audit_entries.len(), 2);
    }

    #[test]
    fn test_supervisor_worker_output_is_structured() {
        struct StructuredWorker;
        impl WorkerAgent for StructuredWorker {
            fn name(&self) -> &str {
                "structured"
            }
            fn execute(&self, input: &str) -> Result<String, String> {
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

    #[test]
    fn test_supervisor_no_domain_logic() {
        struct DomainWorker;
        impl WorkerAgent for DomainWorker {
            fn name(&self) -> &str {
                "domain"
            }
            fn execute(&self, input: &str) -> Result<String, String> {
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
        assert_eq!(result.step_outputs["process"], "processed-domain-data");
    }

    #[test]
    fn test_worker_independent_of_supervisor() {
        let worker = EchoWorker {
            name: "standalone".into(),
        };
        let output = worker.execute("hello").unwrap();
        assert_eq!(output, "echoed: hello");
        assert_eq!(worker.name(), "standalone");
    }

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
}
