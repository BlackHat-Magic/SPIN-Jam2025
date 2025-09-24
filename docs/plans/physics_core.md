# Physics Core Plan

## TDD Log
- **[Red | 2025-09-24]** `tests/docs.rs::physics_design_doc_captures_core_decisions` fails. Current physics design doc lacks explicit statements about integration method, solver strategy, and related decisions.
- **[Green | 2025-09-25]** `tests/physics.rs::physics_test_world_initializes_with_defaults` passes after introducing `PhysicsTestWorld` with deterministic defaults and accessor helpers.

## Design Outline
- **Rigid Bodies & Forces:**
  - Integrate semi-implicit Euler integration (already selected in `physics_design.md`) with per-body mass/inertia properties.
  - Bodies maintain linear/angular velocity; accumulate external forces (gravity, impulses) before integration.
  - Expose hooks to freeze bodies (static/kinematic) via zero inverse mass or explicit flags.
- **Collider Shapes:**
  - Phase 1 priority: spheres, axis-aligned boxes (AABB), oriented boxes (OBB), capsules. Static triangle mesh proxies handled via BVH acceleration.
  - Spheres and capsules rely on analytic distance queries; boxes leverage Separating Axis Theorem (SAT) for overlap tests.
  - Shared trait `ColliderShape` providing support functions (GJK) and bounding volumes (AABB) for broad-phase.
- **Collision Detection:**
  - Broad-phase: deterministic sweep-and-prune over x-axis with optional secondary axis fallback; derived from `PhysicsTestWorld` seed to keep ordering stable.
  - Narrow-phase: GJK/EPA for convex shapes, SAT optimizations for boxes, capsule vs. capsule analytic solver. Manifold generation caches contact normals and penetration depths for warm starting.
  - Contact persistence cache keyed by body pair IDs to reduce jitter.
- **Constraint Solver:**
  - Sequential impulse solver using Gauss–Seidel iteration with Baumgarte stabilization.
  - Supports friction and restitution, with warm-started impulses from previous frames.
  - Constraint graph partitioned into islands; sleeping determined by velocity thresholds and contact island state.
- **ECS Integration:**
  - Components: `RigidBody`, `Collider`, `Velocity`, `AngularVelocity`, `ForceAccumulator`, `PhysicsMaterial`, `Joint`, `Sleeping`. Optional marker `PhysicsProxy` used for entities that mirror the native physics handle.
  - Resources: `PhysicsWorld` (stores bodies, colliders, manifolds), `BroadPhase`, `NarrowPhaseCache`, `ConstraintSolverState`, `PhysicsTime` (fixed-step accumulator), `PhysicsEvents` (collision/contact channels), `PhysicsDebugSettings` (toggles visual overlays).
  - Systems registered through `PhysicsPlugin`:
    - `sync_ecs_to_physics` (`SystemStage::PreUpdate`): reads ECS component state, writes into `PhysicsWorld`, accumulates forces, queues collider changes.
    - `run_physics_step` (`SystemStage::Update`): advances fixed-step loop, executes broad-phase, narrow-phase, solver, updates internal events/resources.
    - `sync_physics_to_ecs` (`SystemStage::PostUpdate`): writes back `Transform`, `Velocity`, and raises events to gameplay systems.
    - `emit_physics_events` (`SystemStage::PostUpdate`): publishes structured collision/contact events using ECS channels.
  - Deterministic order is enforced via stage scheduling and explicit `After` labels in the dispatcher so render/input systems observe authoritative physics state.
- **Math Utilities:**
  - `glam` helpers for inertia tensors, quaternion normalization, matrix decompositions.
  - Provide epsilon-aware comparisons for collision tolerances; instrumentation macros to detect NaNs post-solver.

## Harness Notes
- `PhysicsTestWorld` now seeds RNG deterministically, manages bodies, energy, and integration steps (see `tests/physics.rs`).
- Next fixtures: extend harness to differentiate static/kinematic bodies, add manifold fixtures once collision routines land, and expose penetration depth diagnostics.

## Milestone Deliverables (Phase 1)
- **Milestone 1 – Rigid Body Foundations**
  - Deliverables: `RigidBody`, `Collider`, `Velocity`, `ForceAccumulator` components with registration; `PhysicsWorld` resource storing body handles; gravity + external force accumulation.
  - Acceptance: unit tests cover body creation/removal and gravity integration; harness scenario validates energy consistency for single-body free-fall; documentation updates in `physics_core.md` TDD log.
- **Milestone 2 – Deterministic Broad-Phase**
  - Deliverables: sweep-and-prune structure producing candidate pairs, AABB generation per collider, deterministic ordering seeded from `PhysicsWorld`.
  - Acceptance: tests assert repeatable pair ordering for randomized fixtures seeded via `PhysicsTestWorld`; benchmarks recorded to ensure O(n log n) scaling on sample data.
  - Status: `PhysicsWorld::rebuild_broad_phase` maintains a deterministic sweep-and-prune pair list driven by entity IDs. New tests in `tests/physics.rs` verify repeatable ordering and basic overlap coverage. Next work: integrate axis sweep caching to minimize per-frame rebuild cost and incorporate broad-phase pairs into narrow-phase dispatch.
- **Milestone 3 – Narrow-Phase & Manifolds**
  - Deliverables: GJK/EPA implementation for convex shapes, SAT optimizations for boxes, analytic capsule checks, manifold cache.
  - Acceptance: regression tests using harness fixtures validate contact normals/penetration depth; deterministic manifold IDs reused across frames; docs enumerate supported shape pairs.
- **Milestone 4 – Sequential Impulse Solver**
  - Deliverables: constraint graph construction, warm-starting, Baumgarte stabilization, friction/restitution material parameters.
  - Acceptance: stacked-box harness test asserts positional drift bounds; impulse history stored for debugging; integration tests ensure solver converges within configured iterations.
- **Milestone 5 – ECS Synchronization & Events**
  - Deliverables: full `PhysicsPlugin` system chain, event/resource propagation, post-step transform sync, debug toggles.
  - Acceptance: integration tests confirm render scene matches physics positions after ticks; mock gameplay system receives collision events; documentation includes usage samples for registering physics systems.

## Open Questions
- How should the physics world resource be structured to cooperate with existing `SystemStage` ordering?
- Which initial shapes should be prioritized for prototype coverage?
