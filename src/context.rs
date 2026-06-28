use crate::NbclEngine;
use crate::evaluate::Evaluator;
use crate::registry::Registry;
use std::ops::Deref;
use std::path::PathBuf;

/// Partial Context of registry to retreive only currently evaluated file.
#[derive(Clone)]
pub struct Context(pub(crate) Registry);

impl Deref for Context {
    type Target = Registry;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Context {
    /// Helpful in providing better error diagnostics.
    pub fn get_current_file(&self) -> Option<PathBuf> {
        self.0.current_file.clone()
    }

    pub fn extend(&mut self, other: Context) {
        self.0.extend(other.0);
    }
}

/// Full evaluation context of Evaluator
#[derive(Clone)]
pub struct EvalContext(pub(crate) Evaluator);

impl EvalContext {
    /// Create a fresh evaluator from default [`NbclEngine`] settings.
    pub fn new() -> EvalContext {
        let engine = NbclEngine::new();
        Self::from(&engine)
    }

    /// Create an evaluation context from [`NbclEngine`] (preserves engine metadata).
    pub fn from(engine: &NbclEngine) -> EvalContext {
        let evaluator = Evaluator::new(
            engine.registry.clone(),
            engine.module_resolver.clone(),
            engine.max_depth.clone(),
        );

        EvalContext(evaluator)
    }

    /// This function is helpful in providing better error diagnostics.
    pub fn get_current_file(&self) -> Option<PathBuf> {
        self.0.registry.current_file.clone()
    }

    /// Extend the evaluation context with another one.
    pub fn extend(&mut self, other: EvalContext) {
        self.0.extend(other.0);
    }
}
