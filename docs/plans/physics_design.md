# Klaus of Death Physics Design

## Target Features
- Rigid body dynamics with semi-implicit Euler integration for linear and angular motion, tuned for stability under high iteration counts.
- Collider primitives: spheres, axis-aligned boxes, oriented boxes, capsules; triangle mesh proxies for static geometry.
- Broad-phase collision detection using deterministic sweep-and-prune along principal axes to keep pair ordering stable across platforms.
- Narrow-phase contact generation with GJK/EPA for convex shapes and SAT fallbacks.
- Constraint and joint solver using a sequential impulse solver supporting fixed, hinge, and character controller constraints.
- Hooks for animation-driven events (ragdoll activation, root-motion impulses) that can enqueue impulses into the physics world.

## ECS Integration
- Components: `RigidBody`, `Collider`, `Velocity`, `Acceleration`, `ForceAccumulator`, `Joint`, `PhysicsMaterial`.
- Resources: `PhysicsWorld`, `BroadPhase`, `NarrowPhaseCache`, `ConstraintSolverState`.
- System scheduling:
  - `SystemStage::PreUpdate`: accumulate forces, sync ECS components into physics world snapshot, prepare warm starting caches.
  - `SystemStage::Update`: run integration steps, broad-phase, narrow-phase, constraint solver with warm starting and island sleeping decisions.
  - `SystemStage::PostUpdate`: write back resolved transforms/velocities to ECS `Transform` components and emit events.
- Deterministic fixed-step time accumulator resource to decouple rendering frame rate from physics tick rate.

## Math Utilities
- `glam`-based helpers for quaternion normalization, inertia tensor transforms, and stable matrix decompositions.
- SIMD-friendly vector wrappers for batch operations inside the solver.
- Debug assertions for NaN/INF detection after each major solver phase.

## Testing Strategy
- Establish a reusable `PhysicsTestWorld` helper that constructs deterministic worlds with configurable gravity, integration step, and collision primitives.
- Extend `tests/physics.rs` with energy conservation regression tests that assert total kinetic + potential energy stays within tolerance over fixed iterations.
- Seed broad-phase randomness (if any) with deterministic seeds so ordering remains stable across runs and platforms.
- Provide fixtures for common setups (single body drop, stacked boxes, jointed ragdoll) to accelerate scenario authoring and reuse assertions across suites.

## Open Questions
- Should sleeping bodies be managed via island sleeping heuristics or simple velocity thresholds?
- Desired extensibility for custom joint types (user-defined constraints)?
- How to expose physics debug visualization hooks to the render module?
- What level of determinism is required across platforms (floating-point epsilon policies)?
