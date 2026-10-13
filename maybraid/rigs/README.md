# Rigs

Helpers for socketing and posing named armatures.

Domain crates own recipes, skeleton catalogs (`character-rigs` humanoid tables, firearm kit slots), and the strings they put in [`RigKey`](src/bone_map.rs). This crate owns the runtime loop:

1. **Membership** — nested hosts walk to [`AssemblyRoot`](src/member.rs)
2. **Bone map** — [`RigRoot`](src/bone_map.rs) indexes named descendants, stopping at nested [`AssemblyHost`](src/member.rs)s
3. **Sockets** — [`SocketRef`](src/socket.rs) parents a host under a bone, or under the assembly root when no armature exists yet
4. **Pose** — capture bind TRS, apply [`ResolvedRigPose`](src/pose.rs) layers; skip rotation on [`PoseSkipRotation`](src/pose.rs) so a clip mailbox can own those joints. [`BoneScale::length`](src/pose.rs) is local Y — kit authors align mesh +Y to that axis rather than remapping at fulfill (firearm slots: [`guns/README.md`](../art/items/guns/README.md)).

Add [`RigPlugin`](src/plugin.rs). LOD apps should order [`RigSystems::Membership`](src/plugin.rs) after chunk fulfill.
