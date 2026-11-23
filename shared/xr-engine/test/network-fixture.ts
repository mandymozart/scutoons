// the test experience of network.test.ts and write-xr-vector.ts
/** a small experience: a tap rule on a node, a variable set in one scene and read in the next, a goto between them */
export const doc = {
	id: 'demo',
	configuration: {
		experience: { id: 'demo', title: 'Demo' },
		scenes: [
			{ id: 'poster', label: 'Poster', surfaces: [{ id: 'a3' }], logic: { rules: [{ on: 'keyRattled', do: [{ goto: 'scene:back' }] }] } },
			{ id: 'back', label: 'Back', surfaces: [{ id: 'a3' }], logic: { rules: [{ on: 'sceneChanged', when: 'hasKey', do: [{ show: 'target:door/door' }] }] } }
		],
		entries: [
			{ id: 'cat', title: 'Cat', scene: 'poster', target: { id: 'cat', entity: { ref: 'overlay' } } },
			{ id: 'door', title: 'Door', scene: 'back', target: { id: 'door', entity: { type: 'group', name: 'frame', assets: [], children: [{ type: 'model', name: 'door', assets: [] }] } } }
		],
		entities: {
			overlay: {
				type: 'group',
				name: 'overlay',
				assets: [],
				children: [
					{
						type: 'model',
						name: 'head',
						assets: [],
						components: [{ type: 'behaviour', rules: [{ on: 'tap', do: [{ set: { hasKey: true }, scope: 'experience' }, { emit: 'keyRattled' }] }] }]
					}
				]
			}
		}
	},
	plugins: [{ id: 'spin', api: '3.1', code: 'export const register = (api) => { api.events.on("keyRattled", () => {}); };' }]
};
