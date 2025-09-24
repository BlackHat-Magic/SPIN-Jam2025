# Physics Core Plan

## TDD Log
- **[Red | 2025-09-24]** `tests/docs.rs::physics_design_doc_captures_core_decisions` fails. Current physics design doc lacks explicit statements about integration method, solver strategy, and related decisions.

## Design Outline (Pending)
- **Rigid Bodies & Forces:** _pending drafting_
- **Collider Shapes:** _pending drafting_
- **Collision Detection:** _pending drafting_
- **Constraint Solver:** _pending drafting_
- **ECS Integration:** _pending drafting_
- **Math Utilities:** _pending drafting_

## Open Questions
- How should the physics world resource be structured to cooperate with existing `SystemStage` ordering?
- Which initial shapes should be prioritized for prototype coverage?
