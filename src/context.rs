use crate::registry::Registry;
use std::path::PathBuf;
use std::ops::Deref;

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
