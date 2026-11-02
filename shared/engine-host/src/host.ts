// host.ts (#298): the controller with the bundler's files – what an app creates.
import { EngineHost } from './controller.ts';
import { engineFiles } from './files.ts';
import type { EngineHostOptions } from './options.ts';

export function createEngineHost(o: EngineHostOptions): EngineHost {
	return new EngineHost(o, engineFiles);
}
