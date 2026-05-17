# Supervisor Architecture Implementation Plan

## Overview

Implement a Supervisor (Orchestrator) Architecture pattern for multi-agent task delegation.
A single `Supervisor` agent manages workflow execution by delegating steps to specialized
`WorkerAgent` implementations, tracking state, handling faults, and maintaining an audit trail.

## Test Cases

### 1. Supervisor Lifecycle & Composition
- **TC1:** Supervisor creates with a registered set of named worker agents
- **TC2:** Supervisor rejects creation with no workers (empty team)
- **TC3:** Supervisor rejects duplicate worker names

### 2. Sequential Delegation
- **TC4:** Supervisor executes a 2-step sequential workflow, passing output of step 1 as input to step 2
- **TC5:** Supervisor executes a 3+ step chain, verifying each step's output is captured in order
- **TC6:** Supervisor returns the assembled final result from all step outputs

### 3. Conditional Branching
- **TC7:** Supervisor routes to Worker A when step 1 output satisfies a condition
- **TC8:** Supervisor routes to Worker B when step 1 output fails the condition
- **TC9:** Supervisor halts the workflow early when a terminal condition is met

### 4. Fault Handling
- **TC10:** Supervisor retries a failed worker up to N attempts before propagating
- **TC11:** Supervisor routes to a backup/fallback agent when primary worker fails
- **TC12:** Supervisor marks the workflow as failed but returns partial results when a non-critical step fails
- **TC13:** Supervisor propagates failure immediately when a critical step fails

### 5. State Management & Checkpointing
- **TC14:** Supervisor persists workflow state after each completed step
- **TC15:** Supervisor resumes from a checkpoint, skipping already-completed steps
- **TC16:** Corrupted/invalid checkpoint causes the supervisor to restart from scratch

### 6. Structured Communication
- **TC17:** Worker output must conform to a structured schema; supervisor rejects free-form output
- **TC18:** Supervisor passes structured context (not raw text) between workers

### 7. Audit Trail
- **TC19:** Supervisor records an ordered log of: step name, worker agent, input, output, timestamp
- **TC20:** Audit trail is returned as part of the workflow result
- **TC21:** Audit trail persists across checkpoint/resume cycles

### 8. Separation of Concerns
- **TC22:** Supervisor contains no domain logic — only routing, state, and decision logic
- **TC23:** Worker agents are independently testable without the supervisor

### 9. Dynamic Planning (LLM-driven)
- **TC24:** Supervisor can generate a plan from a high-level goal using the LLM
- **TC25:** Generated plan is validated against registered workers before execution
- **TC26:** Invalid plan step (references unregistered worker) is rejected before execution

## Implementation Phases

| Phase | Test Cases | Description |
|-------|------------|-------------|
| 1 | TC1-TC6 | `WorkerAgent` trait, `Supervisor` struct, sequential delegation, audit trail |
| 2 | TC7-TC9 | Conditional branching with halt support |
| 3 | TC10-TC13 | Retry, fallback agent, critical vs non-critical steps |
| 4 | TC14-TC16 | State checkpointing (serialize/resume) |
| 5 | TC17-TC21 | Structured I/O schema validation, audit persistence |
| 6 | TC24-TC26 | LLM-driven dynamic plan generation and validation |

## New Types

- `WorkerAgent` trait — `execute(&self, input: &str) -> Result<String, String>`, `name(&self) -> &str`
- `WorkflowStep` — name, worker name, condition (optional closure), critical flag, max retries, fallback agent
- `WorkflowStatus` — Running, Completed, Failed, Halted
- `WorkflowState` — current step index, step outputs map, status
- `AuditEntry` — step name, worker, input, output, status, timestamp
- `WorkflowResult` — final output, audit entries, status, step outputs
- `Supervisor` — workers map, steps, state, audit log; `new()`, `builder()`, `execute()`, `resume()`

## Key Design Decisions

- Workers are stateless — all state lives in the `Supervisor`
- Condition closures are `Box<dyn Fn(&str) -> bool>` (not serializable until Phase 4)
- Supervisor is the single point of fault handling
- Strict separation: supervisor coordinates, workers execute domain logic
