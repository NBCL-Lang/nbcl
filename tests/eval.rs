use nbcl::{NativeNodeSchema, PropValidation, Type, Value};
use std::sync::{Arc, Mutex};

#[test]
fn test_native_fn_registration() {
    let call_count = Arc::new(Mutex::new(0));
    let call_count_captured = Arc::clone(&call_count);

    let mut engine = nbcl::NbclEngine::new();
    engine.register_native_fn("increment", vec![], Type::Null, move |_| {
        let mut count = call_count_captured.lock().unwrap();
        *count += 1;
        Ok(Value::Null)
    });

    let file = engine.parse_str("increment() increment()").unwrap();
    engine.evaluate(file).unwrap();

    assert_eq!(*call_count.lock().unwrap(), 2);
}

#[test]
fn test_complex_logic_and_scoping() {
    let mut engine = nbcl::NbclEngine::new();
    engine.register_node(NativeNodeSchema {
        type_name: "Result".into(),
        enforce_id: false,
        validation: PropValidation::Loose,
        child_count: None,
    });

    let code = r#"
        local base_cpu = 2
        local multiplier = 4
        local total_cpu = (base_cpu + 2) * multiplier / 2 # (4 * 4) / 2 = 8
        
        local is_valid = total_cpu == 8 && true
        
        # Test truthy/falsy coalescing
        local version = null || "v1.0"
        
        Result "check" {
            cpu = total_cpu
            valid = is_valid
            ver = version
        }
    "#;

    let file = engine.parse_str(code).unwrap();
    let resolved = engine.evaluate(file).unwrap();

    let node = &resolved.root_nodes[0];
    assert_eq!(node.props.get("cpu").unwrap(), &Value::Int(8));
    assert_eq!(node.props.get("valid").unwrap(), &Value::Bool(true));
    assert_eq!(node.props.get("ver").unwrap(), &Value::Str("v1.0".into()));
}
