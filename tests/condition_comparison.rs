use specguard::baseline::*;
#[test]
fn only_same_typed_unit_can_be_ordered() {
    let min = |v, u: &str| Condition::IntegerMinimum {
        value: v,
        unit: u.into(),
    };
    let max = |v| Condition::IntegerMaximum {
        value: v,
        unit: "ms".into(),
    };
    assert_eq!(
        compare_condition(&min(2, "count"), &min(3, "count")),
        Comparison::Tightened
    );
    assert_eq!(compare_condition(&max(2), &max(3)), Comparison::Relaxed);
    assert_eq!(
        compare_condition(&min(2, "s"), &min(2000, "ms")),
        Comparison::Review
    );
    assert_eq!(
        compare_condition(
            &Condition::Text {
                text: "fast".into()
            },
            &Condition::Text {
                text: "quick".into()
            }
        ),
        Comparison::Review
    );
}
