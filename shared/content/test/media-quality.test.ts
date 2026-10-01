// Unit tests of the media quality setting (media-quality.ts, agents/DESIGN.md "Media quality").
//   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { autoQuality, effectiveQuality, isMediaQualitySetting, pickQualityVariant, qualityForMbps, variantLevel } from '../src/index.ts';

const video = { src: 'v.720p.mp4', variants: { low: 'v.480p.mp4', medium: 'v.720p.mp4', high: 'v.1080p.mp4' } };

test('pickQualityVariant: the level, else the nearest lower, else src', () => {
	assert.equal(pickQualityVariant(video, 'low'), 'v.480p.mp4');
	assert.equal(pickQualityVariant(video, 'medium'), 'v.720p.mp4');
	assert.equal(pickQualityVariant(video, 'high'), 'v.1080p.mp4');
	assert.equal(pickQualityVariant({ src: 'd.mp4', variants: { low: 'l.mp4' } }, 'high'), 'l.mp4');
	assert.equal(pickQualityVariant({ src: 'd.mp4', variants: { high: 'h.mp4' } }, 'medium'), 'd.mp4');
	assert.equal(pickQualityVariant({ src: 'd.mp4', variants: { medium: '' } }, 'medium'), 'd.mp4');
	assert.equal(pickQualityVariant({ src: 'd.mp4' }, 'low'), 'd.mp4');
	assert.equal(variantLevel(video), 'medium');
	assert.equal(variantLevel({ src: 'x.mp4' }), null);
});

test('autoQuality: data saver / slow connections → low; a 4G-class connection → null (measure)', () => {
	assert.equal(autoQuality(null), null);
	assert.equal(autoQuality({}), null);
	assert.equal(autoQuality({ saveData: true, effectiveType: '4g', downlink: 10 }), 'low');
	assert.equal(autoQuality({ effectiveType: '3g', downlink: 10 }), 'low');
	assert.equal(autoQuality({ effectiveType: '4g', downlink: 0.35 }), 'low');
	assert.equal(autoQuality({ effectiveType: '4g', downlink: 10 }), null);
	assert.equal(autoQuality({ effectiveType: '4g', downlink: 1.45 }), null);
	assert.equal(qualityForMbps(0), 'low');
	assert.equal(qualityForMbps(NaN), 'low');
	assert.equal(qualityForMbps(7), 'high');
});

test('effectiveQuality / isMediaQualitySetting', () => {
	assert.equal(effectiveQuality('auto', null), 'low');
	assert.equal(effectiveQuality('auto', 'high'), 'high');
	assert.equal(effectiveQuality('medium', 'low'), 'medium');
	assert.ok(isMediaQualitySetting('auto'));
	assert.ok(!isMediaQualitySetting('ultra'));
	assert.ok(!isMediaQualitySetting(undefined));
});
