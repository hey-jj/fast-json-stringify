//! Error types for building and running a serializer.
//!
//! Messages reproduce the source text byte for byte so callers can match on
//! them. [`BuildError`] is raised while compiling a schema. [`StringifyError`]
//! is raised while serializing a value.

use std::fmt;

/// An error raised while compiling a schema into a serializer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildError {
    message: String,
}

impl BuildError {
    /// Wrap a message produced during the build. Crate-internal so callers
    /// cannot mint an error of this type.
    pub(crate) fn new(message: impl Into<String>) -> Self {
        BuildError {
            message: message.into(),
        }
    }

    /// The error message. This is the stable contract for the cause, and it
    /// matches the text a JavaScript build would throw.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for BuildError {}

/// An error raised while serializing a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringifyError {
    message: String,
}

impl StringifyError {
    /// Wrap a message produced during serialization. Crate-internal so callers
    /// cannot mint an error of this type.
    pub(crate) fn new(message: impl Into<String>) -> Self {
        StringifyError {
            message: message.into(),
        }
    }

    /// The error message. This is the stable contract for the cause, and it
    /// matches the text a JavaScript serialize would throw.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Build the `The value "X" cannot be converted to a TYPE.` message used by
    /// the numeric and date serializers.
    pub(crate) fn cannot_convert(value: impl Into<String>, target: &str) -> Self {
        StringifyError::new(format!(
            "The value \"{}\" cannot be converted to {}.",
            value.into(),
            target
        ))
    }

    /// Build the `"KEY" is required!` message.
    pub(crate) fn required(key: &str) -> Self {
        StringifyError::new(format!("\"{key}\" is required!"))
    }

    /// Build the `The value of 'REF' does not match schema definition.` message.
    pub(crate) fn no_match(reference: &str) -> Self {
        StringifyError::new(format!(
            "The value of '{reference}' does not match schema definition."
        ))
    }

    /// Build the `Item at N does not match schema definition.` message.
    pub(crate) fn item_mismatch(index: usize) -> Self {
        StringifyError::new(format!("Item at {index} does not match schema definition."))
    }
}

impl fmt::Display for StringifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for StringifyError {}
