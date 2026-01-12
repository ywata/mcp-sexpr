//! Example error types demonstrating typed error design patterns.
//!
//! These error types demonstrate best practices for designing
//! strongly-typed error enums using `thiserror`.
//!
//! # Design Principle
//!
//! **"Define enum to convey meaning so that specification changes produce compile-time errors"**
//!
//! By using typed enums instead of strings, you get:
//! - Compile-time verification of error handling
//! - Exhaustive pattern matching
//! - Clear documentation of failure modes
//! - Easier refactoring when requirements change

#![allow(missing_docs)]

use thiserror::Error;

/// Example: Errors related to state management operations.
///
/// This enum demonstrates how to model different failure modes in a state machine
/// or resource management system.
///
/// # Usage
///
/// ```rust
/// use mcp_tools::errors::StateError;
///
/// fn find_resource(id: &str) -> Result<String, StateError> {
///     if id.is_empty() {
///         return Err(StateError::NotFound(id.to_string()));
///     }
///     Ok("resource".to_string())
/// }
/// ```
#[derive(Debug, Error, Clone)]
pub enum StateError {
    /// Resource with the given ID was not found
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Resource is not in the expected state for the operation
    #[error("Resource {resource_id} is not in {expected} state (current: {actual})")]
    InvalidState {
        resource_id: String,
        expected: String,
        actual: String,
    },

    /// Dependency not satisfied for resource
    #[error("Dependency {dep} not satisfied for resource {resource}")]
    DependencyNotSatisfied { resource: String, dep: String },

    /// All dependencies are not complete
    #[error("Resource {0} has incomplete dependencies")]
    IncompleteDependencies(String),

    /// No resources are ready to be processed
    #[error("No resources are ready (all blocked or complete)")]
    NoResourcesReady,

    /// Resource is already in progress
    #[error("Resource {0} is already in progress")]
    AlreadyInProgress(String),

    /// Multiple errors occurred
    #[error("Multiple errors occurred: {0:?}")]
    MultipleErrors(Vec<String>),

    /// Duplicate resource ID
    #[error("Resource with ID {0} already exists")]
    DuplicateId(String),

    /// Transition error (converted from TransitionError)
    #[error("Transition error: {0}")]
    TransitionError(#[from] TransitionError),

    /// Internal lock was poisoned due to a prior panic
    #[error("Internal lock poisoned: {lock}")]
    LockPoisoned { lock: String },
}

/// Example: Errors related to state transitions.
///
/// This enum demonstrates how to model state machine transition errors
/// with detailed context about what went wrong.
///
/// # Usage
///
/// ```rust
/// use mcp_tools::errors::TransitionError;
///
/// fn transition_state(id: &str, from: &str, to: &str) -> Result<(), TransitionError> {
///     if from == to {
///         return Err(TransitionError::InvalidTransition {
///             resource: id.to_string(),
///             from: from.to_string(),
///             to: to.to_string(),
///         });
///     }
///     Ok(())
/// }
/// ```
#[derive(Debug, Error, Clone)]
pub enum TransitionError {
    /// Resource not found when attempting transition
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Resource is not in expected state (cannot transition)
    #[error("Resource '{resource_id}' is not in {expected} state (current: {current})")]
    NotInExpectedState {
        resource_id: String,
        expected: String,
        current: String,
    },

    /// Dependency not found
    #[error("Dependency '{dep}' not found for resource '{resource}'")]
    DependencyNotFound { resource: String, dep: String },

    /// Dependency not satisfied (not in required state)
    #[error("Resource '{resource}' dependency '{dep}' not satisfied (state: {state})")]
    DependencyNotSatisfied {
        resource: String,
        dep: String,
        state: String,
    },

    /// Cannot transition from current state
    #[error("Cannot transition resource '{resource}' from {from} to {to}")]
    InvalidTransition {
        resource: String,
        from: String,
        to: String,
    },
}

/// Example: Errors related to dependency resolution.
///
/// This enum demonstrates how to model dependency graph errors,
/// particularly useful for build systems, task schedulers, or workflow engines.
///
/// # Usage
///
/// ```rust
/// use mcp_tools::errors::DependencyError;
///
/// fn check_dependencies(resource: &str, deps: &[String]) -> Result<(), DependencyError> {
///     if deps.iter().any(|d| d == resource) {
///         return Err(DependencyError::CircularDependency);
///     }
///     Ok(())
/// }
/// ```
#[derive(Debug, Error, Clone)]
pub enum DependencyError {
    /// Circular dependency detected
    #[error("Circular dependency detected in resource dependencies")]
    CircularDependency,

    /// Dependency cycle involving specific resources
    #[error("Dependency cycle detected involving resources: {0:?}")]
    CycleDetected(Vec<String>),

    /// Referenced dependency does not exist
    #[error("Resource '{resource}' depends on non-existent resource '{dep}'")]
    DependencyNotFound { resource: String, dep: String },

    /// Invalid dependency relationship
    #[error("Resource '{resource}' has invalid dependency '{dep}'")]
    InvalidDependency { resource: String, dep: String },

    /// Dependency on descendant (creates cycle)
    #[error("Resource '{resource}' cannot depend on its descendant '{dep}'")]
    DependencyOnDescendant { resource: String, dep: String },

    /// No resources are ready to execute
    #[error("No resources are ready to execute (all blocked or complete)")]
    NoReadyResources,

    /// Topological sort failed
    #[error("Failed to determine execution order: {0}")]
    TopologicalSortFailed(String),
}

/// Example: Validation errors for input data.
///
/// This enum demonstrates how to model validation errors with detailed
/// information about what validation failed and why.
#[derive(Debug, Error, Clone)]
pub enum ValidationError {
    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Invalid value for field
    #[error("Invalid value for {field}: {reason}")]
    InvalidValue { field: String, reason: String },

    /// Value out of range
    #[error("Value for {field} out of range: {value} (expected {min}..{max})")]
    OutOfRange {
        field: String,
        value: String,
        min: String,
        max: String,
    },

    /// Duplicate identifier
    #[error("Duplicate identifier: {0}")]
    DuplicateId(String),

    /// Invalid format
    #[error("Invalid format for {field}: {reason}")]
    InvalidFormat { field: String, reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_error_display() {
        let err = StateError::NotFound("test-id".to_string());
        assert_eq!(err.to_string(), "Resource not found: test-id");
    }

    #[test]
    fn test_transition_error_conversion() {
        let trans_err = TransitionError::NotFound("test".to_string());
        let state_err: StateError = trans_err.into();
        assert!(matches!(state_err, StateError::TransitionError(_)));
    }

    #[test]
    fn test_dependency_error_display() {
        let err = DependencyError::CircularDependency;
        assert_eq!(
            err.to_string(),
            "Circular dependency detected in resource dependencies"
        );
    }

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError::MissingField("name".to_string());
        assert_eq!(err.to_string(), "Missing required field: name");
    }
}
