# App Horizon

Draft concept for owner review · 2026-11-27

Build the parts that make SCUTOONS a better place to author and assemble a toon. Reuse specialist tools for making assets. Keep the team free to draw, shoot, animate, write, model and compose in the tools they choose. The sole developer maintains a small set of integrations that serve several workflows.

## InApp → Spores → Generators → Drivers

This is a map of responsibilities. SCUTOONS already has an MCP bridge to its tools and an address model for event wiring. The first integration to explore is exposing selected addressed event inputs through that bridge. An agent can use existing tools for context, then send an event to an in-app Spore whose wiring invokes a generator. Uploading a hand-made asset is also a complete path through InApp. [MCP bridge](../MCP.md) · [Addressed wiring](../../deck/space/src/rig/address.rs)

| Layer | SCUTOONS owns | Where to stop |
|---|---|---|
| InApp | The scene, shot, character or asset slot. The brief, references, candidate preview, artist’s choice and accepted asset version. Use the existing library and import path. | Add a control when it removes a recurring production step. Keep drawing, editing, modeling and music production in the team’s chosen tools. |
| Spores | The existing wiring and address model. Use explicit production events where a result needs to trigger something in the toon or its authoring flow. | Start with a direct action for a single request. Add a wire when the creator actually needs to connect it to another event. Keep provider waiting and retries in the integration layer. |
| Generators | A named capability the team understands, such as “draft a shot still from these approved references.” Declare the inputs, output type and where a candidate belongs. A copied brief and manual import can prove the capability. | Introduce a shared interface when a workflow has repeated and its inputs are known. Let each discipline refine its own recipe. |
| Drivers | A small adapter for a proven integration. Translate inputs, submit or retrieve a job when supported, surface its status and return a candidate to SCUTOONS. | Let specialist providers do generation, and reuse their job mechanisms where available. Avoid a second general-purpose queue, visual workflow editor or agent runtime. |

**Terminology.** “Generator” here means an asset production capability, as in the storyboard research. The deck also has runtime event generators such as pulse and arpeggio. Keep those distinct. “Driver” here is a proposed term for the adapter that talks to an outside tool or agent. It does not imply a new package, graph, or API. [Storyboard generators](GENERATORS.md) · [Deck event generators](../../deck/events/src/generators/mod.rs) · [Spores wiring](../../deck/space/src/rig/address.rs)

## What earns in-house development

- Build the SCUTOONS part when it protects the source or destination of creative work, such as keeping a candidate attached to the right shot and preserving the artist’s accepted version.
- Keep a recipe as a team-owned workflow while the steps, prompts or tools are still changing. Reuse the existing workflows directory to record examples the creatives actually want to repeat. A recipe can involve manual work, an agent, or a provider. [Production workflow index](../../workflows/README.md)
- Add a driver when the team repeats the workflow and can point to time lost copying inputs, retrieving results or attaching them to the right place. Reuse the current generator or MCP path wherever it already supplies the capability. [SCUTOONS MCP](../MCP.md)
- Consider n8n or another external workflow runner later, when running more jobs or coordinating work across tools becomes a real bottleneck. At that point, test one production workflow against its manual baseline. SCUTOONS retains the accepted assets and creative decisions so the runner can change. [n8n execution history](https://docs.n8n.io/build/understand-workflows/understand-executions/view-all-executions.md)

## First proof

For the technical proof, pick one existing in-app Spore input that can request a generator candidate. Send it from MCP using the existing address format, alongside a request ID, typed payload and target revision. Resolve and authorize the target in the receiving host using the existing role checks. Verify that an MCP request and the same in-app action reach the same handler. Start with one explicitly exposed input. If the app or target is unavailable, return that state to the caller. If tool discovery is needed, list compatible exposed inputs with their payload contracts.

Use one real shot. An illustrator or video artist takes the existing brief and approved references into their preferred tool, brings back a candidate and chooses whether to use it. If useful, hand the exact approved version to the next discipline. Record how much active time went into creating, reviewing and moving files between tools. Fix the largest repeatable SCUTOONS friction first. [Existing shot workflow](../../workflows/scenes/generate-shots.md)

If the first integration is asynchronous, keep the request ID, target revision and returned candidate together. Test a late result and a duplicate completion before using that driver on live work. A late candidate stays available for review against its original request. Only the artist’s acceptance changes the chosen asset. This follows the existing proposal-first direction. [Automation concept](AUTOMATION.md)

## Handoff for planning

Review the boundaries with the owner and one creative who will use the workflow. Confirm the first real shot and identify the existing Spore input, Generator and import path for the proof. Check the current MCP tool registration and event delivery path before choosing where to implement the address bridge. This concept proposes no build tickets or release commitments.
