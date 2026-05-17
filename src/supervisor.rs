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
            if !self.audit_entries.last().unwrap().success {
                if step.critical {
                    self.status = WorkflowStatus::Failed;
                    break;
                }
                // Non-critical: continue to next step
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
