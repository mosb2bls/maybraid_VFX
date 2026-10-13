# RFC-1252: Ambient Wind and Dust VFX

**Status:** Approved  
**Author:** Russell Wang  
**Date:** 13 October 2026

## Table of Contents

- [1: Motivation](#1-motivation)
- [2: Prior Art](#2-prior-art)
- [3: Design](#3-design)
  - [3.1 Scope and visual direction](#31-scope-and-visual-direction)
  - [3.2 Establish the runtime approach](#32-establish-the-runtime-approach)
  - [3.3 Houdini authoring and visual review](#33-houdini-authoring-and-visual-review)
  - [3.4 Ambient wind](#34-ambient-wind)
  - [3.5 Dust emission and movement](#35-dust-emission-and-movement)
  - [3.6 Movement disturbance and boundaries](#36-movement-disturbance-and-boundaries)
  - [3.7 Parameters, transitions and saved configurations](#37-parameters-transitions-and-saved-configurations)
  - [3.8 Performance approach](#38-performance-approach)
  - [3.9 Code delivery and Maybraid API adaptation](#39-code-delivery-and-maybraid-api-adaptation)
- [4: Implementation milestones](#4-implementation-milestones)
- [5: Validation and implementation artifacts](#5-validation-and-implementation-artifacts)
- [6: Decisions to confirm during implementation](#6-decisions-to-confirm-during-implementation)

## 1: Motivation

Maybraid needs reusable wind and dust effects for ambient airflow, short action bursts and trails around moving herds. The effects should be visually coherent, configurable by gameplay code and practical to run together in representative scenes.

This RFC proposes an authoring and runtime approach that combines Houdini visual development with Bevy/Hanabi playback. A general Bevy demonstration will establish the visual behavior and controls before adaptation to the actual Maybraid VFX API. Runtime changes should share one control path across the demonstration panel and the game integration.

## 2: Prior Art

The [Maybraid VFX library](https://github.com/ramate-io/maybraid/tree/main/maybraid/vfx/lib) provides existing effect definitions, layered rendering, spawning and lifecycle management. It is the integration starting point for this design.

[Hanabi](https://docs.rs/bevy_hanabi/0.19.0/bevy_hanabi/) provides GPU particle simulation and configurable initialization, update and rendering modifiers. Houdini provides editable shape, texture and motion authoring.

Specific API and rendering references appear beside the relevant design decisions below. The code snippets are illustrative design excerpts; proposed helper names and final shader integration will be validated during implementation.

## 3: Design

### 3.1 Scope and visual direction

The project covers one ambient wind effect and one reusable dust effect, with configurations for short action bursts and continuous herd dust. Delivery includes a runnable Bevy playground, a parameter panel, editable assets and code, and adaptation to the Maybraid VFX API.

The visual direction combines low-poly characteristics with some realism: irregular shapes, subtle shading, natural movement and restrained earth tones. The shared references and the established dust visual style will guide the design. Wind will carry faint haze and drifting particles. Dust will remain light and semi-transparent, with visible depth as the camera moves around it and enough transparency to reveal the environment behind it.

### 3.2 Establish the runtime approach

Development begins with a small Bevy/Hanabi playground using simplified assets. The first tests will cover camera orbiting, transparent overlap, movement-driven disturbance, ground and obstacle constraints, and multiple simultaneous effects. Results on macOS and Windows will establish the runtime approach and initial emission budgets before detailed visual production.

The dust prototype will combine lightweight irregular 3D lobes with finer mist particles. Expansion, rotation and gentle deformation will create rolling movement; shaded regions, edge breakup and fading will establish depth and softness. The prototype will determine the useful balance between mesh geometry and particle detail, including close-camera transparency.

Rust will define the effects, expose controls and manage spawning, updates and cleanup. Hanabi will handle GPU particle simulation. Custom Hanabi update modifiers and WGSL will support the agreed movement, constraints and material behavior where required. Hanabi's [expression and modifier workflow](https://docs.rs/bevy_hanabi/0.19.0/bevy_hanabi/graph/expr/index.html) provides the mechanism for generating this GPU logic. The 3D lobe path will be assessed separately for update and rendering cost.

### 3.3 Houdini authoring and visual review

Once the initial runtime approach is established, Houdini will support shape, texture and motion development. Editable controls will group the main shape, variation, shading and movement settings. Exported textures and lightweight meshes will supply the runtime effects as needed.

The initial review includes 2–3 Houdini screenshots per base effect, including contextual views. Short videos will then show ambient wind, action-burst dust and continuous dust. Each preview will identify its rendering environment. Bevy previews will accompany this work so that decisions about appearance can be checked against actual playback.

Approved assets will be brought into Bevy and refined alongside the simulation and materials. Final visual acceptance will use the Bevy results.

### 3.4 Ambient wind

Wind will continuously emit faint haze and drifting particles within a configurable region. A shared airflow direction, gentle speed variation and small local deviations will create coherent, natural movement. Particles leaving the emission region will continue drifting and fade smoothly; stopping the emitter will let existing particles finish their lifetime.

Direction, speed, intensity and emission-area controls will support different environments. Runtime changes will use a configurable transition time. Direction will rotate smoothly toward the requested heading, while existing particles will approach the updated flow through acceleration and damping.

### 3.5 Dust emission and movement

Both dust configurations will share the core assets, shading and movement controls:

- **Action burst:** a short emission from a circular region with adjustable radius. Dust spreads outward along the ground, with configurable expansion speed and upward lift, then dissipates.
- **Continuous herd dust:** emission from one movable region with adjustable size. The region follows its source; emitted dust remains in world space, drifts and fades into a trail. Emission scales with source speed, uses modest random variation and respects the configured count limit. At rest, new emission stops and existing dust dissipates.

Continuous emission will use the following illustrative accumulator logic. Carrying fractional counts between frames keeps low-speed emission consistent; the available capacity limits actual spawning.

```text
rate = min(source_speed * emission_factor, max_emission_rate)
spawn_accumulator += rate * dt
requested_count = floor(spawn_accumulator)
spawn_accumulator -= requested_count
spawn_count = min(requested_count, available_capacity)
```

The controller will distribute new dust across the moving region with modest variation. Requests beyond capacity will be dropped for that frame, keeping later emission free of accumulated catch-up bursts.

For a continuous source at steady state, active particle count is approximately **emission rate × average lifetime**, capped by the allocated capacity. A two-second average lifetime gives the following initial test sweep for the fine-particle layer:

| Emission rate per source | Approximate active particles per source | Across 10 sources |
|---|---:|---:|
| 5 particles/second | 10 | 100 |
| 10 particles/second | 20 | 200 |
| 20 particles/second | 40 | 400 |

These are illustrative starting budgets for testing. Mesh lobes will be counted separately, and wind and dust will be tuned to their own appearance and coverage.

Perceived density does not increase linearly with particle count. Coverage, particle size, opacity, lifetime and overlap all contribute; additional particles provide diminishing visual benefit as the effect becomes crowded. At one pixel, overlapping layers with the same opacity `a` produce a combined opacity of approximately `1 - (1 - a)^n`.

[NVIDIA's particle rendering guide](https://developer.nvidia.com/gpugems/gpugems3/part-iv-image-effects/chapter-23-high-speed-screen-particles) explains the associated overdraw and screen-coverage cost. [Hanabi's capacity documentation](https://docs.rs/bevy_hanabi/0.19.0/bevy_hanabi/struct.EffectAsset.html#method.new) explains GPU allocation limits. The initial sample will compare counts and frame times to select a useful visual budget.

Each dust element will carry its own age, initial size and motion variation. Expansion, rise, rolling and drift will evolve over its lifetime. The group controls will separate **emission duration**, meaning how long the source generates dust, from **dust dissipation time**, meaning how long each emitted dust element remains visible.

### 3.6 Movement disturbance and boundaries

The first version supports one moving influence source, represented by a movable test object in the playground. The gameplay system supplies its position and velocity. Adjustable radius, strength and falloff will control how nearby dust is pushed outward and carried along the movement direction. Damping will soften the response, and the influence will diminish smoothly as movement stops.

The same influence data will guide the 3D lobes and finer particles so that they respond coherently. These responses provide visual feedback; character movement and forces remain controlled by the gameplay system.

The following WGSL-style pseudocode outlines the local response for a Hanabi custom update modifier. A smoothed `movement_weight` approaches zero when the influence stops; `push_accel` and `carry_rate` control repulsion and entrainment. Input validation and timestep handling will be completed in the runtime implementation.

```wgsl
let offset = particle.position - influence.position;
let distance = length(offset);
let outward = offset / max(distance, 0.001);
let falloff = 1.0 - smoothstep(0.0, max(influence.radius, 0.001), distance);
let weight = falloff * movement_weight * influence.strength;
let acceleration = weight * (
    outward * push_accel
    + (influence.velocity - particle.velocity) * carry_rate
);
particle.velocity += acceleration * dt;
particle.velocity *= exp(-drag * dt);
```

Hanabi's [update modifier entry point](https://docs.rs/bevy_hanabi/0.19.0/bevy_hanabi/struct.EffectAsset.html#method.update) supports applying this logic during simulation. The lobe implementation will use the same inputs and response model.

Each dust group will support a ground plane defined by position and normal, together with up to two simplified obstacle bounds using spheres or fixed-orientation boxes. Contacts will correct penetration, remove inward velocity and retain damped movement along the surface. This allows dust to slide along ground and obstacles while retaining its overall drift.

### 3.7 Parameters, transitions and saved configurations

The playground panel and API will use a shared set of effect parameters, with documented defaults and useful ranges.

| Group | Main controls |
|---|---|
| Wind | Direction, speed, intensity, emission region, transition time |
| Dust appearance | Initial size, color, density, dissipation time |
| Dust motion | Outward spread, upward lift, rolling strength, drift direction and speed |
| Emission | Burst radius, continuous region size, emission duration, speed-to-emission response |
| Interaction | Influence position and velocity, radius, strength and falloff; ground and obstacle bounds |
| Budget | Emission-rate limits and active-element capacity |

Live wind and motion changes will use frame-rate-independent smoothing. A target/current-value approach, with a blend factor derived from elapsed time and transition time, will keep transitions consistent across different frame rates.

The following Rust excerpt illustrates exponential smoothing for continuous scalar values such as speed, intensity and emission rate:

```rust
let blend = 1.0 - (-dt / transition_time.max(0.001)).exp();
current_value += (target_value - current_value) * blend;
```

Here, `transition_time` controls the response timescale. Color channels will follow the same blend in linear RGB, preserving the relative light and dark shading within the agreed natural palette. Wind direction will use rotational interpolation. Density changes will smoothly adjust emission rate; initial-size changes will apply to newly emitted dust, while existing elements retain their authored growth and fade.

The panel will support saving and loading configuration files. Delivery includes default configurations for ambient wind, action bursts and continuous herd dust.

#### Proposed configuration and runtime inputs

The following Rust excerpt illustrates the proposed data structure. The final types will extend this pattern to the full control set above and follow the project's conventions.

```rust
use bevy::prelude::*;

struct DustSettings {
    initial_size: f32,
    color: LinearRgba,
    emission_rate: f32,
    emission_duration: Option<f32>, // None: emit until stopped
    dissipation_time: f32,
}

struct DustRuntimeInputs {
    emitter_position: Vec3,
    influence_position: Option<Vec3>,
    influence_velocity: Vec3,
    influence_radius: f32,
    influence_strength: f32,
    ground_origin: Vec3,
    ground_normal: Vec3,
}
```

The panel edits `DustSettings`; configuration files store those settings and the remaining effect controls. `DustRuntimeInputs` receives current world-space data from the playground or gameplay adapter, with sphere/box bounds supplied alongside it. Updates will validate ranges and normalize direction inputs before applying them.

Hanabi properties will carry shared values into the GPU simulation. Property names will be declared when building the effect with [`ExprWriter::add_property`](https://docs.rs/bevy_hanabi/0.19.0/bevy_hanabi/graph/expr/struct.ExprWriter.html#method.add_property) and read by the corresponding update modifiers. An absent influence position disables movement disturbance for that instance.

[`EffectProperties`](https://docs.rs/bevy_hanabi/0.19.0/bevy_hanabi/properties/struct.EffectProperties.html) stores values per effect instance and uploads changes through Bevy's render extraction. The adapter will update changed inputs, while the emission controller manages spawn rate and duration. Maybraid's [existing particle properties](https://github.com/ramate-io/maybraid/blob/main/maybraid/vfx/lib/src/particles.rs) provide a reference for this mapping.

Initial size and lifetime will be sampled when particles are created; live influence and color values will be read during updates. Both the panel and Maybraid adapter will feed this same control path.

### 3.8 Performance approach

This pass will target playable operation with approximately ten instances of each base effect in a representative scene. The instance mixture, resolution and test configuration will be agreed during the initial sample review.

Performance work will focus on tuning emission rates, lifetimes and element capacities; limiting transparent overlap; simplifying expensive material details where needed; and sharing mesh and material resources where practical. [Bevy's instancing example](https://bevy.org/examples/shaders/custom-shader-instancing/) provides the reference for resource sharing and batching. Particle movement will remain GPU-driven, with the mesh-lobe path checked separately for update and rendering cost.

The demonstration will expose effect-count and emission controls so that visual quality and frame time can be compared. Optimization will be directed toward the agreed instance test and approved appearance.

### 3.9 Code delivery and Maybraid API adaptation

The general Bevy playground will provide reusable effect definitions, the parameter panel, saved configurations, a movable influence object and demonstrations of spawning, updating, stopping and cleanup. Its structure can use an independent Bevy application or Maybraid abstractions, depending on the initial setup.

The wind and dust effects will be adapted to Maybraid's VFX API. The [existing spawn interface](https://github.com/ramate-io/maybraid/blob/main/maybraid/vfx/lib/src/spawn.rs) exposes `VfxSpawn`, `spawn_vfx` and the `SpawnVfxExt` command extension, using `EffectDefinition` to describe the layers. These provide the starting point for registering the effects and mapping the shared controls to Maybraid's conventions.

Integration will proceed by registering the wind and dust definitions, applying spawn-time settings, wiring live property updates, and implementing continuous-source start/stop and burst cleanup. The adapter will expose the moving influence and boundary inputs, followed by a runnable check of spawning, updates and lifecycle behavior inside Maybraid.

The following usage sketch combines the existing spawn call with proposed update and stop helpers. `dust_definition` represents the new registered continuous-dust effect. The helpers' names and signatures will be finalized during integration.

```rust
// Existing Maybraid spawn interface, using the new effect definition.
let instance = commands.spawn_vfx(
    &dust_definition,
    VfxSpawn {
        transform: Transform::from_translation(position),
        ..default()
    },
);

// Proposed API: apply shared settings and current world-space inputs.
update_vfx(&mut commands, instance, &settings, &runtime_inputs);

// Proposed API: stop emission; let existing elements finish fading.
stop_vfx_emission(&mut commands, instance);
```

Maybraid's [current lifecycle systems](https://github.com/ramate-io/maybraid/blob/main/maybraid/vfx/lib/src/lifecycle.rs) advance instance age and clean up timed effects. Continuous-source integration will keep active sources alive, then clean up their members once emission has stopped and the final emitted elements have dissipated. Burst instances will retain automatic cleanup after playback. Instance ownership will route updates to the relevant particle and lobe layers.

The delivered code consists of Rust effect and control code, the Bevy demonstration and API adapter, plus the WGSL/modifier code used by the final implementation. The parameter panel and API will operate on the same settings.

## 4: Implementation milestones

### 4.1 Technical sample and initial visual previews

Build a simplified Bevy/Hanabi sample demonstrating dust depth, transparency, movement disturbance and boundary behavior, with initial multi-instance checks on macOS and Windows. Produce initial Houdini screenshots and establish emission budgets.

**Review:** confirm the runtime approach and select the direction for shape, color, scale and density before detailed visual production.

### 4.2 Visual direction approval

Refine the selected style using short Houdini videos of wind, action-burst dust and continuous dust. Update the Bevy sample to reflect the selected direction.

**Review:** confirm appearance, movement, intensity and readability for the complete effects.

### 4.3 Complete effects and general Bevy demonstration

Bring the approved assets and movement into the complete Bevy effects. Implement the shared parameter panel and configuration saving/loading, together with spawning, live updates, continuous emission, stopping and cleanup. Tune emission budgets and transparent overlap for the agreed multi-instance test on macOS and Windows.

Produce the runnable general Bevy demonstration, editable Houdini sources, runtime assets and code, saved configurations and test results.

**Validation:** the effects match the selected visual direction; controls and saved configurations work; depth, movement disturbance and boundary behavior are demonstrated; and approximately ten instances per base effect remain playable in the agreed test configuration. The general Bevy demonstration will establish these behaviors before Maybraid API adaptation is validated.

### 4.4 Maybraid VFX API integration

Register the wind and dust effects in the actual Maybraid VFX API and adapt the shared controls to its conventions. Provide spawn-time settings, live updates, continuous-source start/stop behavior, burst cleanup, and movement-influence and boundary inputs. Produce the integration changes and a runnable demonstration within Maybraid.

**Validation:** the effects can be spawned, updated, stopped and cleaned up through the API; the controls and visual behavior are preserved; and the demonstration runs with the project's setup, including the agreed multi-instance configuration.

### 4.5 Usage documentation

Provide a concise README covering dependencies and setup, playground startup, panel controls, configuration saving/loading, parameter defaults and ranges, asset specifications, Maybraid API examples and test results. The repository's Nix flake will support environment consistency where applicable.

**Validation:** a developer can run the demonstration, adjust and reload configurations, and follow the API examples to use the effects.

### 4.6 Follow-up: gameplay integration demonstration

A separately confirmed follow-up can connect the effects to one action-triggered dust burst and one moving gameplay source emitting continuous dust. This will bind gameplay events and supply movement data to the VFX controls.

**Validation:** the agreed action triggers its burst, and the moving source produces and stops its continuous trail correctly in a runnable gameplay example. The exact bindings will be established before this follow-up begins.

## 5: Validation and implementation artifacts

Testing will use an M2 Pro MacBook with 16 GB memory and a Windows laptop with an RTX 5080 Laptop GPU. Results will record hardware, resolution, emission settings and simultaneous effect count. Measurements will compare the same scene with effects enabled and disabled, using release builds.

The playground will expose simultaneous instance count and display frame time and particle usage, distinguishing configured capacity from measured active counts where supported. Draw calls, transparent overlap and simulation cost will guide refinements.

Validation scenarios will cover:

- Close camera orbiting to inspect dust depth, softness and transparency.
- A moving influence object passing through dust and producing coherent disturbance.
- Dust responding to the ground plane and simplified obstacles.
- Multiple wind and dust groups running together, targeting approximately ten instances per base effect under the agreed test configuration.

Implementation artifacts include editable Houdini sources, runtime assets, Rust and WGSL/modifier sources, saved configurations, the runnable demonstration and Maybraid API changes. Asset notes will specify the formats, alpha/color handling and relevant playback settings used by the delivered effects.

## 6: Decisions to confirm during implementation

- Confirm the effect mixture represented by approximately ten instances per base effect, including the action-burst and continuous-dust configurations.
- Establish representative test scenes, resolution and the observed frame-time range used to assess playability.
- Select particle and mesh-lobe budgets from the initial sample; the emission-count examples are starting points for testing.
- Finalize the update and stop interface names, and the mapping from instance ownership to particle and lobe layers, against the Maybraid API version used for implementation.

