# Klaus of Death Engine

## Overview
Klaus of Death is a custom Rust game engine under active development for an original narrative-driven project. The repository focuses on building the core runtime foundation—entity/component orchestration, rendering, input processing, and an upcoming bespoke physics stack—so that future gameplay systems can be layered on top without leaking any sensitive lore details.

## Key Features
- **Lightweight ECS kernel (`ecs/`)**
  - Custom scheduler with stage-based execution (`SystemStage::Init` through `SystemStage::Render`).
  - Procedural macros in `ecs/derive/` generate boilerplate for `Component`, `Resource`, and `system!` definitions, including automatic access tracking for parallelism.
- **Rendering pipeline (`src/render/`)**
  - `wgpu`-powered forward renderer with shader hot-loading in debug builds.
  - OBJ model ingestion and GPU resource management via `render::Model`/`ModelHandle`.
  - 2D quad/sprite support and batched overlay rendering, with resources copied by `build.rs`.
- **Input & timing utilities (`src/utils/`)**
  - Centralized input resource with mouse capture, keyboard state tracking, and raw device events.
  - High-resolution frame timing through the `Time` resource.
- **Gameplay bootstrap (`src/main.rs`)**
  - Winit-driven event loop that plugs all engine systems together.
  - Simple demo scene spawning a camera and model, plus free-fly camera controls for inspection.
- **Documentation-first workflow (`docs/`)**
  - Project requirements and phase plans tracked in Markdown.
  - Tests in `tests/docs.rs` assert critical documentation sections to keep design intent synchronized with implementation.

## Repository Layout
- **`src/`** – Engine entry point, runtime systems, and gameplay bootstrap code.
- **`ecs/`** – Standalone ECS crate with scheduler logic, world state, and derive macros.
- **`resources/`** – Runtime assets (models, shaders, sprites) copied into the target directory by `build.rs`.
- **`docs/`** – Requirements tracker and design plans; phase goals are grouped per subsystem.
- **`tests/`** – Integration tests covering ECS staging, timing utilities, and documentation invariants.

## Prerequisites
- **Rust nightly toolchain**
  - The ECS crate enables `#![feature(specialization)]` and targets the 2024 edition. Install via `rustup toolchain install nightly` and run commands with `cargo +nightly …`.
- **`spirv-opt` binary (optional but recommended)**
  - Release builds call `spirv-opt` from the Vulkan SDK when converting WGSL shaders to SPIR-V (`build.rs`). Debug builds read WGSL directly.
- **System libraries for `wgpu`/`winit`**
  - Ensure your platform meets the requirements for GPU surface creation (Vulkan/DirectX/Metal backend support).

## Getting Started
1. **Clone the repository** and enter the project directory.
2. **Build and run the demo** (Nightly toolchain):
   ```bash
   cargo +nightly run
   ```
3. **Controls** (from `control_player` in `src/main.rs`):
   - Left mouse button: capture cursor.
   - `Esc`: release cursor.
   - `WASD`: translate the camera relative to its facing direction.

The window opens with a rotating-free camera framing a test model rendered via the ECS-driven pipeline.

## Development Workflow
- **Follow Test-Driven Development.** `docs/requirements.md` records the canonical Red → Green → Refactor cycles. Update the tracker and relevant design docs after each iteration.
- **Tests**
  - Run the full suite with:
    ```bash
    cargo +nightly test
    ```
  - Key suites:
    - `tests/app.rs` confirms scheduler stage ordering.
    - `tests/time.rs` validates the `Time` resource lifecycle.
    - `tests/docs.rs` enforces presence and content of the physics design documentation.
- **Design documentation**
  - Physics design goals live in `docs/plans/physics_design.md` and should be refined alongside engine work.
  - Upcoming milestones are captured in `docs/plans/physics_core.md` and `next_steps.md`.

## Roadmap Snapshot
- **Phase 1 – Physics Core**
  - Build deterministic broad/narrow phase detection, sequential impulse solver, and ECS integration hooks.
- **Phase 2 – Rendering Upgrades**
  - Expand lighting, shadows, post-processing, animation, and sprite batching.
- **Phase 3 – UI Framework**
  - Develop ECS-driven UI widgets, input routing, and an overlay render pass.
- **Phase 4 – Networking Tools**
  - Wrap Steamworks P2P APIs and design replication and diagnostics layers.
- **Phase 5 – Tooling & Assets**
  - Deliver full documentation, asset pipelines, and scene management improvements.

Refer to `next_steps.md` for granular checklists and current task status.

## Contributing
This project is currently maintained by the original creators. When extending the engine:
- Keep code changes aligned with the documented roadmap.
- Update `docs/requirements.md` and relevant plan files after each TDD cycle.
- Add or update tests to cover new behavior.

## License
Klaus of Death is released under the **GPL-3.0-or-later** license. See `LICENSE` for full details.
