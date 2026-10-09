mod common;
use common::*;
use specguard::graph::*;
use std::collections::BTreeMap;
#[test]
fn mappings_require_bijection_and_acyclicity() {
    assert!(validate_mappings(&BTreeMap::from([(key("a"), key("b"))])).is_ok());
    assert!(
        validate_mappings(&BTreeMap::from([
            (key("a"), key("b")),
            (key("b"), key("a"))
        ]))
        .is_err()
    );
    assert!(
        validate_mappings(&BTreeMap::from([
            (key("a"), key("c")),
            (key("b"), key("c"))
        ]))
        .is_err()
    );
}
