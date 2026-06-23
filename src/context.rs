use crate::NbclEngine;
use crate::evaluate::Evaluator;
use crate::registry::Registry;
use std::ops::Deref;
use std::path::PathBuf;

#[derive(Clone)]
pub struct Context(pub(crate) Registry);

impl Deref for Context {
    type Target = Registry;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Context {
    pub fn get_current_file(&self) -> Option<PathBuf> {
        self.0.current_file.clone()
    }

    pub fn extend(&mut self, other: Context) {
        self.0.extend(other.0);
    }
}

#[derive(Clone)]
pub struct EvalContext(pub(crate) Evaluator);

impl EvalContext {
    pub fn new() -> EvalContext {
        let engine = NbclEngine::new();
        Self::from(&engine)
    }

    pub fn from(engine: &NbclEngine) -> EvalContext {
        let evaluator = Evaluator::new(
            engine.registry.clone(),
            engine.module_resolver.clone(),
            engine.max_depth.clone(),
        );

        EvalContext(evaluator)
    }

    pub fn extend(&mut self, other: EvalContext) {
        self.0.extend(other.0);
    }
}
