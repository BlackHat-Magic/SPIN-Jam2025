# Requirements Tracker

This document mirrors the execution roadmap defined in `next_steps.md`. Each entry is tracked with a status marker:

- `[ ]` — Not started
- `[~]` — In progress
- `[x]` — Completed

All work must follow a strict Test-Driven Development (TDD) loop (Red → Green → Refactor). After each cycle, update this file and any affected plan documents under `docs/plans/`.

---

## Immediate Next Actions

- `[x]` Draft Physics Design Doc  
  • **Tests:** `tests/docs.rs::physics_design_doc_outlines_phase_one_requirements`, `tests/docs.rs::physics_design_doc_captures_core_decisions`  
  • **Plan Doc:** `docs/plans/physics_design.md`
- `[ ]` Set Up Physics Test Harness  
  • **Tests:** `tests/physics.rs`, new fixture modules (to be created)  
  • **Plan Doc:** `docs/plans/physics_core.md`
- `[ ]` Choose Collision Shapes & Solver Approach  
  • **Tests:** Targeted unit tests per shape module (to be enumerated)  
  • **Plan Doc:** `docs/plans/physics_core.md`
- `[ ]` Define ECS Integration Contract  
  • **Tests:** `tests/app.rs`, `ecs/tests/scheduler.rs`, physics integration tests (pending)  
  • **Plan Doc:** `docs/plans/physics_core.md`
- `[ ]` Plan Milestone Deliverables  
  • **Tests:** N/A (documentation task)  
  • **Plan Doc:** `docs/plans/project_management.md`

---

## Phase 1 – Physics Core (Custom Implementation with TDD)

- `[ ]` Build Broad-Phase Structure  
  • **Tests:** `tests/physics.rs::broad_phase_*` (to be authored)  
  • **Plan Doc:** `docs/plans/physics_core.md`
- `[ ]` Implement Narrow-Phase Contact Generation  
  • **Tests:** `tests/physics.rs::narrow_phase_*`, shape-specific suites  
  • **Plan Doc:** `docs/plans/physics_core.md`
- `[ ]` Create Constraint Solver  
  • **Tests:** `tests/physics.rs::constraint_solver_*`, stacking regression cases  
  • **Plan Doc:** `docs/plans/physics_core.md`
- `[ ]` Integrate Physics Step with `Transform` Updates  
  • **Tests:** `tests/app.rs::physics_updates_transform`, functional demos  
  • **Plan Doc:** `docs/plans/physics_core.md`

---

## Phase 2 – Rendering Upgrades

- `[ ]` Extend Lighting & Materials  
  • **Tests:** `tests/render_lighting.rs` (to be created)  
  • **Plan Doc:** `docs/plans/rendering_upgrades.md`
- `[ ]` Implement Shadow Mapping  
  • **Tests:** `tests/render_shadows.rs` (to be created)  
  • **Plan Doc:** `docs/plans/rendering_upgrades.md`
- `[ ]` Introduce Post-Processing Stack  
  • **Tests:** `tests/render_postprocessing.rs` (to be created)  
  • **Plan Doc:** `docs/plans/rendering_upgrades.md`
- `[ ]` Add Skinned Mesh Animation Support  
  • **Tests:** `tests/render_animation.rs` (to be created)  
  • **Plan Doc:** `docs/plans/rendering_upgrades.md`
- `[ ]` Implement Sprite Batching & 2D Overlay Pass  
  • **Tests:** `tests/render_sprites.rs` (to be created)  
  • **Plan Doc:** `docs/plans/rendering_upgrades.md`

---

## Phase 3 – UI Framework

- `[ ]` Define UI ECS Components  
  • **Tests:** `tests/ui_layout.rs` (to be created)  
  • **Plan Doc:** `docs/plans/ui_framework.md`
- `[ ]` Integrate UI Rendering Pass  
  • **Tests:** `tests/ui_render.rs` (to be created)  
  • **Plan Doc:** `docs/plans/ui_framework.md`
- `[ ]` Route Input Events Through UI Layer  
  • **Tests:** `tests/ui_input.rs` (to be created)  
  • **Plan Doc:** `docs/plans/ui_framework.md`

---

## Phase 4 – Networking Tools (Steamworks P2P)

- `[ ]` Wrap Steamworks API for P2P Sessions  
  • **Tests:** `tests/networking_sessions.rs` (to be created)  
  • **Plan Doc:** `docs/plans/networking_tools.md`
- `[ ]` Design Replication Layer Hooks  
  • **Tests:** `tests/networking_replication.rs` (to be created)  
  • **Plan Doc:** `docs/plans/networking_tools.md`
- `[ ]` Provide Networking Diagnostics  
  • **Tests:** `tests/networking_diagnostics.rs` (to be created)  
  • **Plan Doc:** `docs/plans/networking_tools.md`

---

## Phase 5 – Tooling & Assets

- `[ ]` Author Core Documentation Set  
  • **Tests:** N/A (documentation task)  
  • **Plan Doc:** `docs/plans/tooling_and_assets.md`
- `[ ]` Build Asset Pipeline Helpers  
  • **Tests:** `tests/asset_pipeline.rs` (to be created)  
  • **Plan Doc:** `docs/plans/tooling_and_assets.md`
- `[ ]` Track Future Scene Management Enhancements  
  • **Tests:** N/A (planning task)  
  • **Plan Doc:** `docs/plans/tooling_and_assets.md`

---

## Change Log

Record each TDD cycle update here:

- **2025-09-24** – Drafted physics design doc (`docs/plans/physics_design.md`). Added regression test `tests/docs.rs::physics_design_doc_outlines_phase_one_requirements`. Updated execution plan per TDD cycle.
- **2025-09-24** – Expanded physics design decisions covering integration strategy, solver, warm starting, and deterministic broad-phase ordering. Added regression test `tests/docs.rs::physics_design_doc_captures_core_decisions`.
