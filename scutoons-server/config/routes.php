<?php
declare(strict_types=1);

/**
 * Every URL of the server. The public API contract is agents/ITEMS.md; the admin API is docs/API.md "Admin API".
 * API paths are matched without their trailing slash (NormalizePath): `/api/me/` → `/api/me`.
 * PUBLIC URLS printed on physical items (QR codes, certificates) must never change – agents/PLATFORM.md
 * "RULE: printed / public URLs never change": /?c=, /?c=&skip=1, /verify.html (static), /api/validate/,
 * /api/keys/keys.php, and generate/get-signature.php as used by the verify page.
 */

use DI\Container;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\RateLimit\RateLimiter;
use Scutoons\Domain\RequestContext;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Domain\Secrets;
use Scutoons\Http\Controllers\AccessController;
use Scutoons\Http\Controllers\Admin\BatchesController;
use Scutoons\Http\Controllers\Admin\CodesController;
use Scutoons\Http\Controllers\Admin\SystemController;
use Scutoons\Http\Controllers\Admin\UsersController;
use Scutoons\Http\Controllers\AuthController;
use Scutoons\Http\Controllers\AvatarController;
use Scutoons\Http\Controllers\CatalogController;
use Scutoons\Http\Controllers\ContentController;
use Scutoons\Http\Controllers\HealthController;
use Scutoons\Http\Controllers\InventoryController;
use Scutoons\Http\Controllers\KeysController;
use Scutoons\Http\Controllers\LandingController;
use Scutoons\Http\Controllers\LegacyBridge;
use Scutoons\Http\Controllers\Legacy\GenerateController;
use Scutoons\Http\Controllers\MeController;
use Scutoons\Http\Controllers\ProgressController;
use Scutoons\Http\Controllers\ValidateController;
use Scutoons\Http\Middleware\GenerateToken;
use Scutoons\Http\Middleware\RateLimit;
use Scutoons\Http\Middleware\RequireRole;
use Scutoons\Infrastructure\Config\Settings;
use Slim\App;
use Slim\Routing\RouteCollectorProxy;

return static function (App $app): void {
    /** @var Container $c */
    $c = $app->getContainer();
    $settings = $c->get(Settings::class);

    // Public code lookups: every attempt counts against a per-IP limit (code enumeration)
    $lookupLimit = new RateLimit(
        $c->get(RateLimiter::class),
        static fn(Request $request): string => 'lookup:' . $c->get(Secrets::class)->hmac($c->get(RequestContext::class)->clientIp()),
        $settings->int('ACCESS_MAX_PER_IP_HOUR'),
        3600,
        'Too many codes checked – try again later.',
    );

    // TEMPORARY (refactor step 1): modules moved to Slim so far; the rest runs the old scripts (LegacyBridge)
    $movedModules = ['health', 'catalog', 'auth', 'me', 'access', 'inventory', 'avatar', 'progress'];
    $moved = static fn(string $module): bool => in_array($module, $movedModules, true);

    // ── Public API (agents/ITEMS.md) ───────────────────────────────────────────────────────────────────
    if ($moved('health')) $app->get('/api/health', HealthController::class);

    if ($moved('catalog')) {
        $app->any('/api/items[/index.php]', [CatalogController::class, 'items']);
        $app->any('/api/editions[/index.php]', [CatalogController::class, 'editions']);
        $app->any('/api/toons[/index.php]', [CatalogController::class, 'toons']);
        $app->get('/api/{folder:items|editions|toons}/{file:items|editions|toons}.json', [CatalogController::class, 'file']);
    }

    if ($moved('auth')) {
        $app->post('/api/auth/request', [AuthController::class, 'request']);
        $app->post('/api/auth/verify', [AuthController::class, 'verify']);
        $app->post('/api/auth/logout', [AuthController::class, 'logout']);
    }

    if ($moved('me')) {
        $app->get('/api/me', [MeController::class, 'get']);
        $app->put('/api/me', [MeController::class, 'put']);
    }

    if ($moved('access')) {
        $app->map(['GET', 'POST'], '/api/access', [AccessController::class, 'access'])->add($lookupLimit);
        $app->get('/api/code', [AccessController::class, 'code'])->add($lookupLimit);
    }

    if ($moved('inventory')) {
        $app->post('/api/inventory/claim', [InventoryController::class, 'claim']);
        $app->post('/api/inventory/craft', [InventoryController::class, 'craft']);
        $app->post('/api/inventory/grant', [InventoryController::class, 'grant']);
        $app->post('/api/inventory/import', [InventoryController::class, 'import']);
    }

    if ($moved('avatar')) {
        $app->get('/api/avatar', [AvatarController::class, 'get']);
        $app->put('/api/avatar', [AvatarController::class, 'put']);
    }

    if ($moved('progress')) {
        $app->get('/api/progress/{toon:[A-Za-z0-9_-]{1,64}}', [ProgressController::class, 'get']);
        $app->put('/api/progress/{toon:[A-Za-z0-9_-]{1,64}}', [ProgressController::class, 'put']);
    }

    if ($moved('content')) $app->map(['GET', 'PUT', 'POST'], '/api/content/{rest:.+}', ContentController::class);

    // PUBLIC URLS (printed): legacy validation for old viewer builds, public key of the certificate signatures
    if ($moved('validate')) $app->any('/api/validate[/index.php]', ValidateController::class);
    if ($moved('keys')) $app->any('/api/keys/keys.php', KeysController::class);

    // ── Admin API: Session + RequireRole(admin) (docs/API.md "Admin API") ──────────────────────────────
    if ($moved('admin')) {
        $app->group('/api/admin', function (RouteCollectorProxy $admin): void {
            $admin->get('/dashboard', [SystemController::class, 'dashboard']);
            $admin->get('/events', [SystemController::class, 'events']);
            $admin->get('/catalog', [SystemController::class, 'catalog']);
            $admin->get('/keys', [SystemController::class, 'keys']);

            $admin->get('/codes', [CodesController::class, 'list']);
            $admin->post('/codes', [CodesController::class, 'create']);
            $admin->get('/codes/{code}', [CodesController::class, 'get']);
            $admin->get('/codes/{code}/events', [CodesController::class, 'events']);
            $admin->post('/codes/{code}/revoke', [CodesController::class, 'revoke']);
            $admin->post('/codes/{code}/unrevoke', [CodesController::class, 'unrevoke']);
            $admin->get('/codes/{code}/signature', [CodesController::class, 'signature']);
            $admin->put('/codes/{code}/signature', [CodesController::class, 'saveSignature']);

            $admin->get('/batches', [BatchesController::class, 'list']);
            $admin->post('/batches', [BatchesController::class, 'create']);
            $admin->get('/batches/{filename}', [BatchesController::class, 'get']);
            $admin->put('/batches/{filename}', [BatchesController::class, 'save']);
            $admin->get('/batches/{filename}/zip', [BatchesController::class, 'zip']);
            $admin->get('/files/{type}/{filename}', [BatchesController::class, 'getFile']);
            $admin->put('/files/{type}/{filename}', [BatchesController::class, 'putFile']);

            $admin->get('/users', [UsersController::class, 'list']);
            $admin->get('/users/{id}', [UsersController::class, 'get']);
            $admin->put('/users/{id}/roles/{role}', [UsersController::class, 'grant']);
            $admin->delete('/users/{id}/roles/{role}', [UsersController::class, 'revoke']);
            $admin->get('/roles', [UsersController::class, 'roles']);
        })->add(new RequireRole($c->get(Roles::class), Roles::ADMIN));
    }

    // Everything else below /api/ is unknown (ErrorHandler → 404 not_found "Unknown endpoint.")
    $app->any('/api[/{rest:.*}]', $moved('api-done') ? static function (): never {
        throw new \Scutoons\Domain\ApiError(404, 'not_found', 'Unknown endpoint.');
    } : LegacyBridge::class);

    // ── LEGACY generate/ SPA endpoints (static token; removed once the admin app has parity) ────────────
    if ($moved('generate')) {
        $token = $c->get(GenerateToken::class);
        $app->post('/generate/auth.php', [GenerateController::class, 'auth']);
        $app->any('/generate/get-signature.php', [GenerateController::class, 'signature']); // PUBLIC: verify page
        $app->get('/generate/files/signatures/{name:SIGNATURE-[A-Za-z0-9_-]{1,32}\.json}', [GenerateController::class, 'signatureFile']); // verify page fallback
        $app->group('/generate', function (RouteCollectorProxy $generate): void {
            $generate->any('/generate.php', [GenerateController::class, 'generate']);
            $generate->get('/get-batches.php', [GenerateController::class, 'batches']);
            $generate->get('/get-batch.php', [GenerateController::class, 'batch']);
            $generate->get('/download-batch.php', [GenerateController::class, 'download']);
            $generate->post('/files.php', [GenerateController::class, 'files']);
            $generate->any('/save-signature.php', [GenerateController::class, 'saveSignature']);
            $generate->get('/keys.php', [GenerateController::class, 'keys']);
            $generate->get('/stats.php', [GenerateController::class, 'stats']);
            $generate->get('/code-events.php', [GenerateController::class, 'codeEvents']);
            $generate->any('/revoke.php', [GenerateController::class, 'revoke']);
            $generate->get('/users.php', [GenerateController::class, 'users']);
            $generate->any('/user-roles.php', [GenerateController::class, 'userRoles']);
        })->add($token);
    }

    // ── QR landing page: /, /?c=CODE (printed on every QR code), and every other page path ──────────────
    $app->any('/[{path:.*}]', $moved('landing') ? LandingController::class : LegacyBridge::class);
};
