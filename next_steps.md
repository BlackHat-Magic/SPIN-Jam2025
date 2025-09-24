# Next Steps

## Phase 1 – Physics Core (Custom Implementation with TDD)

- **Design Document**
  - Outline target features: rigid bodies, collider shapes, broad/narrow phase collision detection, constraint solver, joints, character controllers, animation-driven physics hooks.
  - Specify ECS integration: components (`RigidBody`, `Collider`, `Velocity`, `Force`, `Joint`, etc.), resources (physics world, manifolds), and scheduling across `SystemStage::PreUpdate`, `SystemStage::Update`, and `SystemStage::PostUpdate`.
  - Detail math utilities leveraging `glam` for vector/quaternion operations and matrix transforms.
- **Testing Strategy**
  - Expand [tests/physics.rs](cci:7://file:///home/slangerosuna/proj/klaus_of_death_again/tests/physics.rs:0:0-0:0) into a TDD harness covering: integration steps, energy conservation bounds, collision responses, joint constraints, deterministic regression scenarios.
  - Provide helper fixtures (e.g., `PhysicsTestWorld`) to build scenes within tests.
  - Use deterministic seeds for any randomized broad-phase structures.
- **Implementation Tasks**
  - Build broad-phase structure (sweep-and-prune or BVH) and unit tests.
  - Implement narrow-phase contact generation for primitive shapes (sphere, box, capsule, mesh proxies).
  - Create constraint solver (iterative impulse-based) with tests for stacking and joint stability.
  - Integrate physics step with ECS `Transform` updates so rendering remains synchronized.

## Phase 2 – Rendering Upgrades

- **Lighting & Materials**
  - Extend lighting model to support multiple dynamic lights, physically-based BRDF improvements, emissive materials.
- **Shadows**
  - Implement directional and point-light shadow mapping (cascaded shadow maps or cube maps) with resource management tests.
- **Post-processing**
  - Introduce post-process pipeline stages (bloom, tone mapping) and configuration via ECS resources.
- **Animation**
  - Add skinned mesh support in `render::model`, including GPU skinning buffers and animation blending systems.
- **2D Rendering**
  - Implement sprite batching and 2D overlay pass sharing GPU resources with 3D pipeline.

## Phase 3 – UI Framework

- **ECS UI Model**
  - Define UI components (nodes, layout containers, style data) and systems for layout calculation, event routing, focus management.
- **Rendering**
  - Integrate UI draw data into overlay pass with batching and text rendering (font atlas management).
- **Input Integration**
  - Route [utils::input::Input](cci:2://file:///home/slangerosuna/proj/klaus_of_death_again/src/utils/input.rs:55:0-64:1) events into UI systems; add tests ensuring event propagation.

## Phase 4 – Networking Tools (Steamworks P2P)

- **Steamworks Integration**
  - Wrap `steamworks` crate with ECS resource managing Steam callbacks and session lifecycle.
- **Replication Layer**
  - Design hooks for state replication and RPC handling tied to ECS component serialization.
- **Diagnostics**
  - Provide debugging utilities (latency stats, packet inspection) with automated tests/mocks where possible.

## Phase 5 – Tooling & Assets

- **Documentation**
  - Produce architecture docs, API reference, and sample projects illustrating workflows.
- **Asset Pipeline**
  - Create import/export guidance for Blender-authored content; consider CLI helpers for asset packaging.
- **Future Scene Management**
  - Keep track of feature requests for optional scene graph/prefab tools once engine core is solid.

## Immediate Next Actions

1. **Draft Physics Design Doc** – Capture all Phase 1 decisions and open questions.
2. **Set Up Physics Test Harness** – Expand [tests/physics.rs](cci:7://file:///home/slangerosuna/proj/klaus_of_death_again/tests/physics.rs:0:0-0:0) with initial scaffolding and utilities.
3. **Choose Collision Shapes & Solver Approach** – Decide on primitive support order and solver method (impulse-based, sequential impulses).
4. **Define ECS Integration Contract** – Document how physics components/resources interact with existing systems (`Transform`, `SystemStage` usage).
5. **Plan Milestone Deliverables** – Break Phase 1 into sprint-sized tasks with acceptance tests for each feature slice.