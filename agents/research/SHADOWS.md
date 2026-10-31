Status: research 2026-10-31 (owner: "also a research topic: high-performing drop and contact shadows") – for deck-space (native + wasm / WebGL2); issue #215.

# Drop and contact shadows for deck-space

**Our case:** the beings float (a drop shadow on the floor tells where they are), they are built from **spheres and
tubes** (node points, the core, the head plate, hoses), the sets are baked rooms (light already in the textures, #74 v2),
and the targets are the X1 (UHD 620) and the S22 in the browser (**WebGL2**: no compute; keep passes and samples low).
Beings' own light (the glow lights the room, #204) may later cast too.

## The methods
| Method | What it gives | Cost | Fit |
|---|---|---|---|
| **Blob / decal shadow** | a soft dark disc (or a squashed ellipse) on the floor under the being, sized / faded by height | one quad per being, a texture or analytic falloff | **phase 1 – the drop shadow**: cheap everywhere, reads the float height |
| **Analytic sphere / capsule occlusion** | soft shadows and ambient occlusion from spheres / capsules, computed in the fragment shader of the receivers (floor, room, other beings) – the "capsule shadows" AAA games use for characters | per receiver fragment × the casters nearby (cull to a few) | **phase 2 – contact-quality soft shadows**: our bodies *are* spheres and tubes, so this is exact for us and needs no shadow map |
| **Planar projected shadow** | the mesh flattened onto the floor plane along the light, drawn dark | one extra draw of the caster | crisp floor shadows for one main light; simple |
| **Shadow maps** (+ PCF / cascades) | general shadows from any geometry for a directional / spot light | a depth pass per light + filtering; cascades for big rooms | only if the sets get dynamic lights casting room geometry – heavy for the X1 / S22 |
| **Screen-space contact shadows** | short ray-march in the depth buffer toward the light – the small-scale contact detail shadow maps miss | per pixel × steps (2–4 suggested; often off on mobile) | an optional post pass (#204) on decks; off on phones |
| **SSAO / GTAO** | ambient occlusion in the corners and under objects | a half-res pass + blur | optional, post (#204); the baked room already has AO |
| **SDF shadows** | soft shadows by marching a signed distance field | a 3D texture / SDF of the room | nice but heavy to build; no |

## Recommendation
1. **Drop shadow (now):** a blob per being on the set's floor (the room's floor height / a raycast down to the room's
   mesh at load), radius and darkness by the float height and the being's size, softened; the conductor's ring = many
   blobs, one draw instanced. Also for scene objects (#210) and cloth (#207) as needed.
2. **Analytic sphere / capsule shadows (next):** the being's core, head and node points as spheres, hoses as capsule
   chains; receivers (the floor / room shader, the other beings) compute soft occlusion from the nearest N casters
   toward the main light direction + a small ambient term. Cull per receiver (a grid of the casters' bounds). This gives
   contact-quality softness where a being hangs low, and costs nothing when beings are far from surfaces.
3. **Screen-space contact shadows (optional):** a post pass for decks with GPU headroom (the X1 test decides), off on
   phones; #204's stack.
4. **Shadow maps only later** if a scene's lights must cast room geometry dynamically.

Everything stays audiovisual-friendly: the shadow's softness / darkness can follow the being's level or glow.

## Sources
[Unity HDRP Contact Shadows](https://docs.unity3d.com/cn/Packages/com.unity.render-pipelines.high-definition@10.4/api/UnityEngine.Rendering.HighDefinition.ContactShadows.html) ·
[Unreal: Contact Shadows](https://dev.epicgames.com/documentation/unreal-engine/contact-shadows-in-unreal-engine) ·
[threepipe SS contact shadows (steps, mobile)](https://threepipe.org/plugin/SSContactShadowsPlugin.html) ·
[Capsule soft shadows for characters (Unity community)](https://discussions.unity.com/t/released-ambient-character-shadows-a-capsule-soft-shadow-implementation/866466) ·
[Unity Graphics PR (contact shadows)](https://github.com/Unity-Technologies/Graphics/pull/8186/files)
