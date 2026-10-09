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

#[test]
fn unsupported_units_and_free_text_always_require_review() {
    for (before, after) in [
        (
            Condition::IntegerMinimum {
                value: 2,
                unit: "unreviewed-unit".into(),
            },
            Condition::IntegerMinimum {
                value: 3,
                unit: "unreviewed-unit".into(),
            },
        ),
        (
            Condition::IntegerMaximum {
                value: 2,
                unit: "unreviewed-unit".into(),
            },
            Condition::IntegerMaximum {
                value: 2,
                unit: "unreviewed-unit".into(),
            },
        ),
        (
            Condition::Text {
                text: "fast".into(),
            },
            Condition::Text {
                text: "fast".into(),
            },
        ),
    ] {
        assert_eq!(compare_condition(&before, &after), Comparison::Review);
    }
}
