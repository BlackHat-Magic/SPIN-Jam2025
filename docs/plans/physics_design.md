# Klaus of Death Physics Design

## Target Features
- Rigid body dynamics with linear and angular motion integration.
- Collider primitives: spheres, axis-aligned boxes, oriented boxes, capsules; triangle mesh proxies for static geometry.
- Broad-phase collision detection using sweep-and-prune along principal axes with deterministic ordering.
- Narrow-phase contact generation with GJK/EPA for convex shapes and SAT fallbacks.
- Constraint and joint solver supporting fixed, hinge, and character controller constraints.
- Hooks for animation-driven physics events (ragdoll activation, root-motion impulses).

## ECS Integration
- Components: `RigidBody`, `Collider`, `Velocity`, `Acceleration`, `ForceAccumulator`, `Joint`, `PhysicsMaterial`.
- Resources: `PhysicsWorld`, `BroadPhase`, `NarrowPhaseCache`, `ConstraintSolverState`.
- System scheduling:
  - `SystemStage::PreUpdate`: accumulate forces, sync ECS components into physics world snapshot.
  - `SystemStage::Update`: run integration steps, broad-phase, narrow-phase, constraint solver.
  - `SystemStage::PostUpdate`: write back resolved transforms/velocities to ECS `Transform` components and emit events.
- Deterministic fixed-step time accumulator resource to decouple rendering frame rate from physics tick rate.

## Math Utilities
- `glam`-based helpers for quaternion normalization, inertia tensor transforms, and stable matrix decompositions.
- SIMD-friendly vector wrappers for batch operations inside the solver.
- Debug assertions for NaN/INF detection after each major solver phase.

## Open Questions
- Should sleeping bodies be managed via island-based heuristics or simple velocity thresholds?
- Desired extensibility for custom joint types (user-defined constraints)?
- How to expose physics debug visualization hooks to the render module?
- What level of determinism is required across platforms (floating-point epsilon policies)?
