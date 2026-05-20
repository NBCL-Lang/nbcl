use crate::registry::Registry;
use std::ops::Deref;

pub struct Context(pub(crate) Registry);

impl Deref for Context {
    type Target = Registry;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
