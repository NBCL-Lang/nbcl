use crate::registry::Registry;
use crate::ast::PropValidation;

/// Register the default nodes present in Nbcl
pub(crate) fn register_builtin_nodes(registry: &mut Registry) {
    // Universal 'Object' node
    registry.add_node(
        "Object",
        true,
        PropValidation::Loose,
    );
}