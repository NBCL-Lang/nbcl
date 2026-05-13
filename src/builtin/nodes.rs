use crate::ast::utils::{NativeNodeSchema, PropValidation};
use crate::registry::Registry;

/// Register the default nodes present in Nbcl
pub(crate) fn register_builtin_nodes(registry: &mut Registry) {
    // Universal 'Object' node
    registry.add_node(NativeNodeSchema {
        type_name: "Object".to_string(),
        enforce_id: false,
        validation: PropValidation::Loose,
        child_count: None,
    });
}
