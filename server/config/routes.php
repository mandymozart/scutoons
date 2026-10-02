<?php
declare(strict_types=1);

/**
 * Every URL of the API (api.scutoons.com). The public API contract is agents/ITEMS.md; the admin API is docs/API.md
 * "Admin API". API paths are matched without their trailing slash (NormalizePath): `/api/me/` → `/api/me`.
 * PUBLIC URLS printed on physical items (QR codes, certificates) must never change – agents/PLATFORM.md
 * "RULE: printed / public URLs never change". The pages (/?c=, /?c=&skip=1, /verify.html) are the qr app (qr/); it
 * proxies /api/validate/, /api/keys/keys.php and the verify page's generate/get-signature.php + signature files to
 * the routes below, byte for byte – so these must never change either.
 */

use DI\Container;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\RateLimit\RateLimiter;
use Scutoons\Domain\RequestContext;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Domain\Secrets;
use Scutoons\Domain\Settings\RuntimeSettings;
use Scutoons\Http\Controllers\AccessController;
use Scutoons\Http\Controllers\Admin\BatchesController;
use Scutoons\Http\Controllers\Admin\ClaimsController as AdminClaimsController;
use Scutoons\Http\Controllers\Admin\CodesController;
use Scutoons\Http\Controllers\Admin\EmailsController;
use Scutoons\Http\Controllers\Admin\FeaturedController;
use Scutoons\Http\Controllers\Admin\ContentController as AdminContentController;
use Scutoons\Http\Controllers\Admin\SettingsController;
use Scutoons\Http\Controllers\Admin\SystemController;
use Scutoons\Http\Controllers\Admin\UsersController;
use Scutoons\Http\Controllers\Admin\VersionsController;
use Scutoons\Http\Controllers\AuthController;
use Scutoons\Http\Controllers\AvatarController;
use Scutoons\Http\Controllers\CatalogController;
use Scutoons\Http\Controllers\CertificatesController;
use Scutoons\Http\Controllers\ClaimsController;
use Scutoons\Http\Controllers\EmbassyController;
use Scutoons\Http\Controllers\EntitiesController;
use Scutoons\Http\Controllers\HealthController;
use Scutoons\Http\Controllers\InventoryController;
use Scutoons\Http\Controllers\KeysController;
use Scutoons\Http\Controllers\LibraryController;
use Scutoons\Http\Controllers\Legacy\GenerateController;
use Scutoons\Http\Controllers\MeController;
use Scutoons\Http\Controllers\NotFoundController;
use Scutoons\Http\Controllers\PageNotFoundController;
use Scutoons\Http\Controllers\ProductsController;
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

    // Public code lookups: every attempt counts against a per-IP limit (code enumeration). The limits are runtime
    // settings (admin app → Settings; 0 = no limit). QR scans (GET /code/?scan=1 from the qr app, trusted proxy) have
    // their own, higher bucket: at an event many people scan over one Wi-Fi, i.e. one public IP.
    $ipHash = static fn(): string => $c->get(Secrets::class)->hmac($c->get(RequestContext::class)->clientIp());
    $isScan = static fn(Request $request): bool => ($request->getQueryParams()['scan'] ?? null) === '1'
        && $c->get(RequestContext::class)->viaTrustedProxy();
    $lookupLimit = new RateLimit(
        $c->get(RateLimiter::class),
        static fn(Request $request): string => 'lookup:' . $ipHash(),
        static fn(Request $request): int => $c->get(RuntimeSettings::class)->lookupLimitPerHour(),
        3600,
        'Too many codes checked – try again later.',
    );
    $codeLimit = new RateLimit(
        $c->get(RateLimiter::class),
        static fn(Request $request): string => ($isScan($request) ? 'qrscan:' : 'lookup:') . $ipHash(),
        static fn(Request $request): int => $isScan($request)
            ? $c->get(RuntimeSettings::class)->scanLimitPerHour()
            : $c->get(RuntimeSettings::class)->lookupLimitPerHour(),
        3600,
        'Too many codes checked – try again later.',
    );

    // ── Public API (agents/ITEMS.md) ───────────────────────────────────────────────────────────────────
    $app->get('/api/health', HealthController::class);

    $app->any('/api/items[/index.php]', [CatalogController::class, 'items']);
    $app->any('/api/editions[/index.php]', [CatalogController::class, 'editions']);
    $app->any('/api/toons[/index.php]', [CatalogController::class, 'toons']);
    $app->get('/api/{folder:items|editions|toons}/{file:items|editions|toons}.json', [CatalogController::class, 'file']);

    // Shop data (config/products.json, config/featured-products.json – was the viewer's static/api/*.json)
    $app->any('/api/products', [ProductsController::class, 'catalog']);
    $app->any('/api/products/featured', [ProductsController::class, 'featured']);

    $app->post('/api/auth/request', [AuthController::class, 'request']);
    $app->post('/api/auth/verify', [AuthController::class, 'verify']);
    $app->post('/api/auth/logout', [AuthController::class, 'logout']);
    // Single sign-on (docs/API.md): who is signed in (cookie or Bearer) · move an app's stored Bearer token into the cookie
    $app->get('/api/auth/session', [AuthController::class, 'session']);
    $app->post('/api/auth/session', [AuthController::class, 'exchange']);

    $app->get('/api/me', [MeController::class, 'get']);
    $app->put('/api/me', [MeController::class, 'put']);

    $app->map(['GET', 'POST'], '/api/access', [AccessController::class, 'access'])->add($lookupLimit);
    $app->get('/api/code', [AccessController::class, 'code'])->add($codeLimit);

    $app->post('/api/inventory/claim', [InventoryController::class, 'claim']);
    $app->post('/api/inventory/craft', [InventoryController::class, 'craft']);
    $app->post('/api/inventory/grant', [InventoryController::class, 'grant']);
    $app->post('/api/inventory/import', [InventoryController::class, 'import']);
    $app->post('/api/inventory/use', [InventoryController::class, 'use']);
    $app->post('/api/effects/end', [InventoryController::class, 'endEffect']);
    // Ownership + gifting (agents/ITEMS.md "Ownership, claims, gifting"): gift an item to another reader, the "you
    // received" notices, ownership claims ("This is mine") with optional evidence files
    $app->post('/api/inventory/gift', [InventoryController::class, 'gift']);
    $app->post('/api/inventory/received/seen', [InventoryController::class, 'seen']);
    $app->post('/api/claims', [ClaimsController::class, 'create']);
    $app->get('/api/claims', [ClaimsController::class, 'list']);
    $app->post('/api/claims/{id:[0-9a-f-]{36}}/files', [ClaimsController::class, 'addFile']);
    $app->post('/api/claims/{id:[0-9a-f-]{36}}/withdraw', [ClaimsController::class, 'withdraw']);

    $app->get('/api/avatar', [AvatarController::class, 'get']);
    $app->put('/api/avatar', [AvatarController::class, 'put']);

    $app->get('/api/progress/{toon:[A-Za-z0-9_-]{1,64}}', [ProgressController::class, 'get']);
    $app->put('/api/progress/{toon:[A-Za-z0-9_-]{1,64}}', [ProgressController::class, 'put']);

    // Content store by entity key – the creator's write path (docs/API.md "Entities", role creator)
    $app->get('/api/entities', [EntitiesController::class, 'index']);
    $app->get('/api/entities/{key:[a-z0-9:_-]+}', [EntitiesController::class, 'get']);
    $app->put('/api/entities/{key:[a-z0-9:_-]+}', [EntitiesController::class, 'put']);
    $app->get('/api/entities/{key:[a-z0-9:_-]+}/versions', [EntitiesController::class, 'versions']);
    $app->post('/api/entities/{key:[a-z0-9:_-]+}/{action:publish|unpublish|restore}', [EntitiesController::class, 'action']);

    // Library: the reader's entities composed from the content store (agents/DATA.md, docs/API.md "Library")
    $app->get('/api/library/toons', [LibraryController::class, 'toons']);
    $app->get('/api/library/toons/{toon}', [LibraryController::class, 'toon']);
    $app->get('/api/library/toons/{toon}/seasons/{number:[0-9]{1,6}}', [LibraryController::class, 'season']);
    $app->get('/api/library/toons/{toon}/chapters/{number:[0-9]{1,6}}', [LibraryController::class, 'chapter']);
    $app->get('/api/library/toons/{toon}/cast', [LibraryController::class, 'cast']);
    $app->get('/api/library/toons/{toon}/credits', [LibraryController::class, 'credits']);
    $app->get('/api/library/characters', [LibraryController::class, 'characters']);
    $app->get('/api/library/map', [LibraryController::class, 'map']);
    $app->get('/api/events', [LibraryController::class, 'events']);

    // PUBLIC URLS (printed): legacy validation for old viewer builds, public key of the certificate signatures
    $app->any('/api/validate[/index.php]', ValidateController::class);
    $app->any('/api/keys/keys.php', KeysController::class);
    // Certificate verification (qr app /verify.html, third parties): checked server-side against the PUBLISHED key and
    // the signature on record; public status only (registered / revoked), never the owner (agents/ITEMS.md "Certificates")
    $app->post('/api/certificates/verify', [CertificatesController::class, 'byFile'])->add($lookupLimit);
    $app->get('/api/certificates[/{code:[^/]{1,300}}]', [CertificatesController::class, 'byCode'])->add($lookupLimit); // or ?code=

    // Embassy (embassy.scutoons.com): partner network, roles liaison / ambassador (checked in Domain\Embassy\Embassy)
    $app->group('/api/embassy', function (RouteCollectorProxy $embassy): void {
        $id = '{id:[0-9a-f-]{36}}';
        $embassy->get('/me', [EmbassyController::class, 'me']);
        $embassy->get('/team', [EmbassyController::class, 'team']);
        $embassy->get('/today', [EmbassyController::class, 'today']);
        $embassy->get('/orgs', [EmbassyController::class, 'list']);
        $embassy->post('/orgs', [EmbassyController::class, 'create']);
        $embassy->get("/orgs/$id", [EmbassyController::class, 'get']);
        $embassy->put("/orgs/$id", [EmbassyController::class, 'update']);
        $embassy->delete("/orgs/$id", [EmbassyController::class, 'delete']);
        $embassy->post("/orgs/$id/people", [EmbassyController::class, 'addPerson']);
        $embassy->put("/orgs/$id/leads/{track:[a-z]{1,16}}", [EmbassyController::class, 'lead']);
        $embassy->post("/orgs/$id/activities", [EmbassyController::class, 'log']);
        $embassy->put("/people/$id", [EmbassyController::class, 'updatePerson']);
        $embassy->delete("/people/$id", [EmbassyController::class, 'deletePerson']);
        $embassy->post('/import', [EmbassyController::class, 'import']);
    });

    // ── Admin API: Session + RequireRole(admin) (docs/API.md "Admin API") ──────────────────────────────
    $app->group('/api/admin', function (RouteCollectorProxy $admin): void {
        $admin->get('/dashboard', [SystemController::class, 'dashboard']);
        $admin->get('/events', [SystemController::class, 'events']);
        $admin->get('/catalog', [SystemController::class, 'catalog']);
        $admin->get('/keys', [SystemController::class, 'keys']);
        $admin->get('/versions', VersionsController::class);
        $admin->get('/settings', [SettingsController::class, 'get']);
        $admin->put('/settings', [SettingsController::class, 'put']);
        // the email module (Domain\Mail): templates + previews, the sent-mail log, the admin notification recipients
        $admin->get('/emails/templates', [EmailsController::class, 'templates']);
        $admin->get('/emails/templates/{id}/preview', [EmailsController::class, 'preview']);
        $admin->get('/emails/log', [EmailsController::class, 'log']);
        $admin->get('/emails/recipients', [EmailsController::class, 'recipients']);
        $admin->put('/emails/recipients', [EmailsController::class, 'saveRecipients']);
        // the editions on the landing page (stored as the setting `featured_products`; GET /api/products/featured/ reads it)
        $admin->get('/featured', [FeaturedController::class, 'get']);
        $admin->put('/featured', [FeaturedController::class, 'put']);

        $admin->get('/codes', [CodesController::class, 'list']);
        $admin->post('/codes', [CodesController::class, 'create']);
        $admin->get('/codes/{code}', [CodesController::class, 'get']);
        $admin->get('/codes/{code}/events', [CodesController::class, 'events']);
        $admin->post('/codes/{code}/revoke', [CodesController::class, 'revoke']);
        $admin->post('/codes/{code}/unrevoke', [CodesController::class, 'unrevoke']);
        $admin->get('/codes/{code}/history', [CodesController::class, 'history']);
        $admin->post('/codes/{code}/transfer', [CodesController::class, 'transfer']);
        $admin->get('/codes/{code}/signature', [CodesController::class, 'signature']);
        $admin->put('/codes/{code}/signature', [CodesController::class, 'saveSignature']);

        // ownership claims (agents/ITEMS.md "Ownership, claims, gifting")
        $admin->get('/claims', [AdminClaimsController::class, 'list']);
        $admin->get('/claims/{id:[0-9a-f-]{36}}', [AdminClaimsController::class, 'get']);
        $admin->post('/claims/{id:[0-9a-f-]{36}}/transfer', [AdminClaimsController::class, 'transfer']);
        $admin->post('/claims/{id:[0-9a-f-]{36}}/reject', [AdminClaimsController::class, 'reject']);
        $admin->get('/claims/{id:[0-9a-f-]{36}}/files/{file:[0-9a-f-]{36}}', [AdminClaimsController::class, 'file']);

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
        $admin->put('/users/{id}/characters/{character}', [UsersController::class, 'unlockCharacter']);
        $admin->delete('/users/{id}/characters/{character}', [UsersController::class, 'lockCharacter']);
        $admin->delete('/users/{id}/characters/{character}/looks/{look}', [UsersController::class, 'lockLook']);
        $admin->get('/roles', [UsersController::class, 'roles']);

        $admin->get('/content', [AdminContentController::class, 'index']);
    })->add(new RequireRole($c->get(Roles::class), Roles::ADMIN));

    // Everything else below /api/ is unknown (ErrorHandler → 404 not_found "Unknown endpoint.")
    $app->any('/api[/{rest:.*}]', NotFoundController::class);

    // ── LEGACY generate/ SPA endpoints (static token; removed once the admin app has parity) ────────────
    $token = $c->get(GenerateToken::class);
    $app->post('/generate/auth.php', [GenerateController::class, 'auth']);
    // PUBLIC: the verify page (qr app, proxied) – these two stay in the API when generate/ retires
    $app->any('/generate/get-signature.php', [GenerateController::class, 'signature']);
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

    // Every other path: 404 (plain text). The QR landing page /?c= and /verify.html live in the qr app (qr/), which
    // proxies /api/validate/, /api/keys/keys.php, the verify page's get-signature.php / signature files (+ /api/*) here.
    $app->any('/[{path:.*}]', PageNotFoundController::class);
};
