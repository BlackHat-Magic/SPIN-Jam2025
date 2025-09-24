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
