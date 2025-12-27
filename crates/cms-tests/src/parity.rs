//! Parity testing framework
//!
//! Compares oxcms output against reference implementations.

use crate::accuracy::{DeltaEStats, compare_rgb_buffers};
use std::fmt;

/// Reference CMS implementation for comparison
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceCms {
    /// lcms2 - industry standard
    Lcms2,
    /// moxcms - pure Rust reference
    Moxcms,
    /// qcms - Firefox's CMS
    Qcms,
    /// skcms - Chrome's CMS (via saved outputs)
    Skcms,
}

impl fmt::Display for ReferenceCms {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReferenceCms::Lcms2 => write!(f, "lcms2"),
            ReferenceCms::Moxcms => write!(f, "moxcms"),
            ReferenceCms::Qcms => write!(f, "qcms"),
            ReferenceCms::Skcms => write!(f, "skcms"),
        }
    }
}

/// Result of a parity test
#[derive(Debug)]
pub struct ParityResult {
    /// Name of the test
    pub test_name: String,
    /// Reference CMS used
    pub reference: ReferenceCms,
    /// DeltaE statistics
    pub delta_e: DeltaEStats,
    /// Whether the test passed
    pub passed: bool,
    /// Optional notes about differences
    pub notes: Option<String>,
}

impl ParityResult {
    /// Check if this result indicates exact match
    pub fn is_exact(&self) -> bool {
        self.delta_e.max < 0.0001
    }

    /// Check if this result is within acceptable tolerance
    pub fn is_acceptable(&self) -> bool {
        self.delta_e.is_acceptable()
    }
}

/// A parity test comparing oxcms to a reference implementation
pub struct ParityTest {
    /// Test name
    pub name: String,
    /// Test description
    pub description: String,
    /// Reference implementation
    pub reference: ReferenceCms,
    /// Source file for the test
    pub source_file: Option<String>,
    /// Whether this test is expected to fail
    pub expected_fail: bool,
    /// Reason for expected failure
    pub expected_fail_reason: Option<String>,
}

impl ParityTest {
    /// Create a new parity test
    pub fn new(name: impl Into<String>, reference: ReferenceCms) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            reference,
            source_file: None,
            expected_fail: false,
            expected_fail_reason: None,
        }
    }

    /// Add a description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Mark as expected to fail
    pub fn expected_fail(mut self, reason: impl Into<String>) -> Self {
        self.expected_fail = true;
        self.expected_fail_reason = Some(reason.into());
        self
    }

    /// Run the parity test with provided buffers
    pub fn run(&self, oxcms_output: &[u8], reference_output: &[u8]) -> ParityResult {
        let delta_e = compare_rgb_buffers(reference_output, oxcms_output);
        let passed = if self.expected_fail {
            !delta_e.is_acceptable()
        } else {
            delta_e.is_acceptable()
        };

        ParityResult {
            test_name: self.name.clone(),
            reference: self.reference,
            delta_e,
            passed,
            notes: self.expected_fail_reason.clone(),
        }
    }
}

/// Collection of parity tests
pub struct ParityTestSuite {
    tests: Vec<ParityTest>,
}

impl ParityTestSuite {
    pub fn new() -> Self {
        Self { tests: Vec::new() }
    }

    pub fn add(&mut self, test: ParityTest) {
        self.tests.push(test);
    }

    pub fn tests(&self) -> &[ParityTest] {
        &self.tests
    }
}

impl Default for ParityTestSuite {
    fn default() -> Self {
        Self::new()
    }
}
