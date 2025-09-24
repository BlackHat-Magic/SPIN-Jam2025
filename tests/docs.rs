use std::fs;

#[test]
fn physics_design_doc_outlines_phase_one_requirements() {
    let path = "docs/plans/physics_design.md";
    let contents = fs::read_to_string(path).expect("physics design document missing");

    for marker in [
        "## Target Features",
        "## ECS Integration",
        "## Math Utilities",
        "## Open Questions",
    ] {
        assert!(
            contents.contains(marker),
            "expected section `{}` in physics design document",
            marker
        );
    }
}

#[test]
fn physics_design_doc_captures_core_decisions() {
    let contents = fs::read_to_string("docs/plans/physics_design.md")
        .expect("physics design document missing");

    for decision in [
        "semi-implicit euler integration",
        "sequential impulse solver",
        "warm starting",
        "island sleeping",
        "deterministic sweep-and-prune",
        "animation-driven events",
    ] {
        assert!(
            contents.to_lowercase().contains(decision),
            "missing design decision `{}` in physics design document",
            decision
        );
    }
}

#[test]
fn physics_design_doc_describes_testing_strategy() {
    let contents = fs::read_to_string("docs/plans/physics_design.md")
        .expect("physics design document missing");

    assert!(
        contents.contains("## Testing Strategy"),
        "expected `## Testing Strategy` section in physics design document"
    );

    for expectation in [
        "PhysicsTestWorld",
        "energy conservation",
        "deterministic seeds",
    ] {
        assert!(
            contents.to_lowercase().contains(&expectation.to_lowercase()),
            "missing testing strategy detail `{}` in physics design document",
            expectation
        );
    }
}
