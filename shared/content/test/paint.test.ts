import { test } from 'node:test';
import assert from 'node:assert/strict';
import { isGradient, paintCss, paintImage } from '../src/paint.ts';

// issue #94: a text box's paint as CSS
test('paintCss: colours stay, gradients become CSS images (radial stops scaled by the radius)', () => {
	assert.equal(paintCss('#ff0000'), '#ff0000');
	assert.equal(paintCss(undefined), null);
	const stops = [{ at: 0, color: '#ffc531' }, { at: 100, color: '#00ff94', opacity: 0.5 }];
	assert.equal(paintCss({ type: 'linear', angle: 180, stops }), 'linear-gradient(180deg, #ffc531 0%, #00ff9480 100%)');
	assert.equal(paintCss({ type: 'radial', center: { x: 50, y: 40 }, radius: 50, stops }), 'radial-gradient(circle farthest-corner at 50% 40%, #ffc531 0%, #00ff9480 50%)');
	assert.equal(paintImage('#fff'), 'linear-gradient(#fff, #fff)');
	assert.equal(isGradient('#fff'), false);
	assert.equal(isGradient({ type: 'linear', angle: 0, stops }), true);
});
