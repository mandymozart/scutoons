// Lets node:test run the package's sources unbundled: OSCT's copy imports without extensions and through the
// aliases its tsconfig maps (`@engine/*`, `@shared/*`, `#lib/*`, `.js` names for `.ts` files). Test-only.
import { register } from 'node:module';

register('./resolve.mjs', import.meta.url);
