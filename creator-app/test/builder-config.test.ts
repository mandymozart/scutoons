import { test } from 'node:test';
import assert from 'node:assert/strict';

process.env.SCUTOONS_BUILDER_DRY = '1';
const { default: config, installerScript } = await import('../electron-builder.config.ts');

test("the installers register the creator's scheme of the one link definition", () => {
	assert.deepEqual(config.protocols, [{ name: 'SCUTOONS Creator', schemes: ['scutoons-creator'] }]);
	const nsh = installerScript('scutoons-creator');
	assert.match(nsh, /WriteRegStr HKCU "Software\\Classes\\scutoons-creator" "URL Protocol" ""/);
	assert.match(nsh, /"\$INSTDIR\\\$\{APP_EXECUTABLE_FILENAME\}" "%1"/);
	assert.match(nsh, /DeleteRegKey HKCU "Software\\Classes\\scutoons-creator"/);
});

test('the app carries the link definition, its config and the brand icon', () => {
	const from = (config.extraResources as { from: string }[]).map((r) => r.from);
	assert.deepEqual(from, ['../server/schema/tables/deck-link.json', 'config.json', '../creator/static/app-icon-512.png']);
});
