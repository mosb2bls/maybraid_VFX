# Mobs

- [`characters/`](characters/): characters define mob NPCs and noisy builders thereof. They include a species, an inventory, and a set of brains. We define four sets of noisy constructors: 
    - **build:** base attribute adjustments for the character. 
    - **species:** defines a species. 
    - **inventory:** generates an appropriate inventory. 
    - **brains:** defines a set of brains within a class hierarchy. 

The user will then access a constructor `MobCharacter<Build, Species, Inventory, Brains>::from_num(num: f32)` that gives the character. 


- [`scenes/`](scenes/): mobs compose rosters of characters and define mob intelligence brains. Hence we typically have `Mob<Roster, Intelligence>`.
- [`barking/`](barking/): groups compose multiple mobs for a supposed mob cell. This is similar to forest cells where mobs are the groves. They differ from forest cells in that they map urbanization dependencies to form selection criteria. 

For this version, we propose to take a direct gen model on available grove and urbanization types from `chico` and `richmond`. This will allow us to work out some initial rules before trying to take a step back and generalize. 

Spawning APIs from groups targeting the Richmond Surface model `Arc<RichmonSurfaceModel>` (forgot if this is the actual name) to determine where reasonable spawn points are. We will likely make this its own crate. I'm not sure if we want to bake the Richmond type in here so that we have the right respawn brain, or if we author a patch system that will apply/make available the Richmond spawn points model at runtime. 

Groups may put multiple mobs over the same area. We typically use large `400m` cells and allocate 2-12 mobs per cell. We may even use multiple groups over the same POI initialization to create a more dynamic feel.

## Implementation

- [`characters/generated`](characters/generated/) provides `MobCharacter<Build, Species, Inventory, Brains>`,
  the resolved `CharacterSceneRecipe`, and `MobCharacterScenesPlugin`. A scene plant
  becomes the physical NPC controller, its real inventory bag and selected firearm,
  the personality intelligence users, and a nested character-model LodScene.
- [`scenes`](scenes/) provides `Mob<Roster, Intelligence>` and the resolved `MobScene`
  semantic LodScene. Its always-on host owns roster, affiliations, POI/journey,
  travel, hunt, tether-lock, and respawn intelligence. Only High emits character
  plants, one chunk at a time; lower levels retain the host brain. The High
  decision follows the traveling host's current transform rather than its
  original spawn anchor.
- [`barking`](barking/) provides `MobGroup`, the five group families, and the
  `MobWorldSample` / `MobWorldHosts` adapter seams. Family still comes from the
  environment sample; Guard / Brawler / Pleb hosts in Frontier, Warfront, and
  Dystopian groups plant on supplied urban-leaf / building disks. Pack members
  keep a ~2 m occupancy disk on High fulfill and death replace, remember the
  last few replace POIs, and meander with a per-slot score salt so they do not
  stack on one pin. `MaybraidWorld` selects sparse 400 m mob cells
  from the live Richmond/Chico model configuration, generates them in a 3 km
  ring, and presents cell → group → `MobScene` hosts in a 1 km ring. Spawn
  elevation is resolved against composed Durham terrain and Richmond pads at
  presentation time and traveling hosts are fitted again as their XZ changes.
  Journeying hosts are `RoutingIntelligenceUser`s: POI goals become a coarse
  Fixed-layer corridor, and `MobTravel` lerps the tether along those hops. High
  emits `RosterRef` stubs under the host LOD
  tree; fulfill spawns character capsules unparented so host travel cannot tow
  them. Plants follow by tether and keep Avian movement for combat; they do not
  inherit host routing.
  `PendingMobGroups` remains available for focused tests and
  manual playground scenes. [`on-terrain-playground`](on-terrain-playground)
  is the pointed Durham patch (one short herd or pack, no world stream).

The world publishes one global POI for each presented grove and urban setting,
local POIs for presented storeys, and a budgeted local vegetation POI for at most
one real plant host per 48 m spatial bucket. Child POIs leave the registry with
their LOD host, so discovery never retains culled scene anchors.

High character plants are `RosterRef` stubs (`ChildOf` the host for cull only).
Fulfill spawns the live body with `MobSlot` + `MobId` off the host transform
tree. `MobSystems::Bind` resolves the live entity, installs the tether context,
and combines character-local POI interests with the mob interest table.
LOD cull despawns the stub, then the still-bound body; death replacement invokes
the same `CharacterSceneRecipe` used by initial High fulfillment and must not
use the cull path. A replacement rejected outside High is released and retried;
the default policy chooses among weighted nearby mob-interest POIs without
immediately repeating one, then uses a deterministic varied host-relative
fallback when none are available.

## Characters
Initial proposed characters are as follows:

### Build

- **Base:** base build.
- **Wraith:** generous buff to speed. Reduction to health.
- **Tank:** generous buff to health. Reduction to speed.
- **Brawler:** generous buff to strength. Reduction to agility.
- **Renegade:** small buffs to damage and speed.
- **Warrior:** small buffs to all stats. 
- **Master:** generous buffs to all stats.

### Species
All species except the fish species are included. 

### Inventory

- **Empty:** no inventory; used for animals. All quadrupeds get this and no other inventory. 
- **Clothed:** a set of clothing items. 
- **Flashy:** a set of rare clothing items.
- **Grunt:** a set of common clothing items and a generic weapon. 
- **Mercenary:** a set of rare clothing items and a rare but generic weapon. 
- **Specialist:** a set of rare clothing items and a rare weapon of a specific type. 
- **Mist:** a set of rare clothing items and a very rare weapon of a specific type. 

## Brains

- **Grazinger:** mostly for quadrupeds, has vegetation local POI. Varied combat skills.
- **Pack Hunter:** has other character local POI (like herd members). Mostly combatitive; sensitive to low health for fleeing. Varied combat skills.
- **Raider:** mostly for bipeds, has POI for other characters (mostly civilians and other raider groups) and town or urban areas. Varied combat skills. 
- **Guard:** bipeds or quadrupeds. Combatitive; sensitive to low health for fleeing. Varied combat skills but rarely low. 
- **Civilian:** bipeds or quadrupeds. Local POI. No long-range movements. Rarely combatitive; mostly fleeing. Low combat skills.
- **Roamer:** general exploring type. Local POI. Varied combat skills.
- **Brawler:** replicates FFA behavior in real world. Creates an FFA saloon. Varied combat skills.

## Mobs

- **Herd:** 1-24 characters. Grazingers and maybe a Roamer or two. Mixed species within. Preference for herbivorous quadrupeds.  
- **Pack:** 3-12 characters. Pack Hunters. One species. Preference for carnivorous quadrupeds.
- **Raider:** 3-12 characters. Raiders and maybe a Roamer. Mixed bipeds within. Preference for bipeds.
- **Guard:** 3-12 characters. Guards and maybe a Roamer. Mixed bipeds or quadrupeds within. Preference for bipeds or quadrupeds. Tending to have a most common species. 
- **Pleb:** 10-24 characters. Civilians and maybe a Roamer.
- **Rambles:** 1-12 characters. Roamers. Mixed species within. Prefence for either quadruped or biped as opposed to mixing across both. 
- **Brawler:** 6-12 characters. Brawlers. Mixed species within. Bipeds. 

## Groups

- **Peaceful:** 2-12 mobs. Prioritize allocatig **Pleb** to urbanization triggers and **Herd** to vegetation triggers. 
- **Wild:** 2-8 mobs. Allocate mix of **Raider**, **Herd**, **Pack**, and **Rambler** to form a balanced group. 
- **Frontier:** 2-12 mobs. Prioritize **Guard**, **Raider**, and **Brawler** on urbanization triggers and **Herd** and **Pack** on vegetation triggers. 
- **Warfront:** 2-12 mobs. Prioritize **Guard**, **Raider**, **Plebs**, and **Brawler** on urbanization triggers and **Herd** and **Pack** on vegetation triggers. 
- **Dystopian:** 2-12 mobs. Prioritize **Guard** and **Pleb** on urbanization triggers and **Herd** on vegetation triggers. 