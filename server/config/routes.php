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
use Scutoons\Http\Controllers\AgentChatsController;
use Scutoons\Http\Controllers\AgentUsageController;
use Scutoons\Http\Controllers\ReviewsController;
use Scutoons\Http\Controllers\AssetsController;
use Scutoons\Http\Controllers\Admin\BatchesController;
use Scutoons\Http\Controllers\Admin\CatalogController as CatalogAdminController;
use Scutoons\Http\Controllers\Admin\ClaimsController as AdminClaimsController;
use Scutoons\Http\Controllers\Admin\CodesController;
use Scutoons\Http\Controllers\Admin\EmailsController;
use Scutoons\Http\Controllers\Admin\NewsletterController as AdminNewsletterController;
use Scutoons\Http\Controllers\Admin\FeaturedController;
use Scutoons\Http\Controllers\Admin\ContentController as AdminContentController;
use Scutoons\Http\Controllers\Admin\SettingsController;
use Scutoons\Http\Controllers\Admin\ServerLogController;
use Scutoons\Http\Controllers\Admin\SystemController;
use Scutoons\Http\Controllers\Admin\UsersController;
use Scutoons\Http\Controllers\Admin\VersionsController;
use Scutoons\Http\Controllers\AuthController;
use Scutoons\Http\Controllers\AvatarController;
use Scutoons\Http\Controllers\CatalogController;
use Scutoons\Http\Controllers\ContactController;
use Scutoons\Http\Controllers\CertificatesController;
use Scutoons\Http\Controllers\ClaimsController;
use Scutoons\Http\Controllers\EmbassyController;
use Scutoons\Http\Controllers\EntitiesController;
use Scutoons\Http\Controllers\FlowController;
use Scutoons\Http\Controllers\HealthController;
use Scutoons\Http\Controllers\InventoryController;
use Scutoons\Http\Controllers\KeysController;
use Scutoons\Http\Controllers\LibraryController;
use Scutoons\Http\Controllers\Legacy\SignatureController;
use Scutoons\Http\Controllers\MeController;
use Scutoons\Http\Controllers\NewsletterController;
use Scutoons\Http\Controllers\NotFoundController;
use Scutoons\Http\Controllers\PageNotFoundController;
use Scutoons\Http\Controllers\ProductsController;
use Scutoons\Http\Controllers\ProgressController;
use Scutoons\Http\Controllers\StockistsController;
use Scutoons\Http\Controllers\ValidateController;
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
    // The public "Where to buy" list: cached by browsers / proxies for 5 minutes, so a reader hits it rarely – the limit
    // only stops scraping loops (per client IP and hour)
    $stockistsLimit = new RateLimit(
        $c->get(RateLimiter::class),
        static fn(Request $request): string => 'stockists:' . $ipHash(),
        600,
        3600,
        'Too many requests – try again later.',
    );

    // The contact form (Domain\Contact\ContactForm): a few messages per client IP and hour – stops bots, not people
    $contactLimit = new RateLimit(
        $c->get(RateLimiter::class),
        static fn(Request $request): string => 'contact:' . $ipHash(),
        5,
        3600,
        'Too many messages – try again later or write to hello@scutoons.com.',
    );

    // ── Public API (agents/ITEMS.md) ───────────────────────────────────────────────────────────────────
    $app->get('/api/health', HealthController::class);

    $app->any('/api/items[/index.php]', [CatalogController::class, 'items']);
    $app->any('/api/editions[/index.php]', [CatalogController::class, 'editions']);
    $app->any('/api/toons[/index.php]', [CatalogController::class, 'toons']);
    $app->get('/api/{folder:items|editions|toons}/{file:items|editions|toons}.json', [CatalogController::class, 'file']);

    // Shop data (tables catalog_products / catalog_featured – was config/products.json, featured-products.json)
    $app->any('/api/products', [ProductsController::class, 'catalog']);
    $app->any('/api/products/featured', [ProductsController::class, 'featured']);
    // Stores that carry SCUTOONS products (the Embassy's listed stores, public fields only – agents/EMBASSY.md "Stores")
    $app->get('/api/stockists', [StockistsController::class, 'list'])->add($stockistsLimit);
    $app->post('/api/contact', [ContactController::class, 'send'])->add($contactLimit);

    $app->post('/api/auth/request', [AuthController::class, 'request']);
    $app->post('/api/auth/verify', [AuthController::class, 'verify']);
    $app->post('/api/auth/logout', [AuthController::class, 'logout']);
    // Single sign-on (docs/API.md): who is signed in (cookie or Bearer) · move an app's stored Bearer token into the cookie
    $app->get('/api/auth/session', [AuthController::class, 'session']);
    $app->post('/api/auth/session', [AuthController::class, 'exchange']);

    // Newsletter (agents/NEWSLETTER.md): the unsubscribe link of every newsletter (GET = a page with one button, POST =
    // unsubscribe, also the mail client's one-click), the batches from the host's cron (no session: a key)
    $app->get('/api/newsletter/unsubscribe', [NewsletterController::class, 'unsubscribePage']);
    $app->post('/api/newsletter/unsubscribe', [NewsletterController::class, 'unsubscribe']);
    $app->map(['GET', 'POST'], '/api/newsletter/cron', [NewsletterController::class, 'cron']);

    $app->get('/api/me', [MeController::class, 'get']);
    $app->put('/api/me', [MeController::class, 'put']);

    $app->map(['GET', 'POST'], '/api/access', [AccessController::class, 'access'])->add($lookupLimit);
    $app->get('/api/code', [AccessController::class, 'code'])->add($codeLimit);

    $app->post('/api/inventory/claim', [InventoryController::class, 'claim']);
    $app->post('/api/inventory/craft', [InventoryController::class, 'craft']);
    $app->post('/api/inventory/grant', [InventoryController::class, 'grant']);
    // a story takes an item (VN takeItem, BACKLOG 10.38)
    $app->post('/api/inventory/take', [InventoryController::class, 'take']);
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
    // where a catalog item is placed in every toon (item limits, BACKLOG 10.39) – role creator
    $app->get('/api/catalog/items/{id:[A-Za-z0-9_.-]{1,100}}/placements', [EntitiesController::class, 'itemPlacements']);
    // the creator's SCUTOONS-wide flow: every toon's ways out of its chapters + dead things (docs/API.md "Flow index", role creator)
    $app->get('/api/creator/flow-index', [FlowController::class, 'index']);
    // Review before publish (docs/API.md "Reviews", role creator; the review rules per toon decide the rest)
    $app->get('/api/reviews', [ReviewsController::class, 'overview']);
    $app->post('/api/reviews', [ReviewsController::class, 'submit']);
    $app->post('/api/reviews/{action:publish|return}', [ReviewsController::class, 'decide']);

    // The creator's Ask panel chat history (docs/API.md "Agent chats", content editors, each only their own chats);
    // the id is any path segment so a malformed one answers 422 invalid_id, not 404
    $app->get('/api/agent-chats', [AgentChatsController::class, 'index']);
    $app->get('/api/agent-chats/{id}', [AgentChatsController::class, 'get']);
    $app->put('/api/agent-chats/{id}', [AgentChatsController::class, 'put']);
    $app->delete('/api/agent-chats/{id}', [AgentChatsController::class, 'delete']);
    // ...and its usage log: one row per AI API call, written by the Ask service with the creator's session AND its
    // X-Service-Key (docs/API.md "Agent usage"); the admin overview is GET /api/admin/agent-usage
    $app->post('/api/agent-usage', [AgentUsageController::class, 'record']);

    // Asset manager (agents/ASSETS.md): the creator's asset picker – browse, look up, upload new files (content editors)
    $asset = '{id:[0-9a-f-]{36}}';
    $app->get('/api/assets', [AssetsController::class, 'list']);
    $app->get('/api/assets/lookup', [AssetsController::class, 'lookup']);
    $app->get('/api/assets/missing', [AssetsController::class, 'missing']);
    $app->get('/api/assets/orphans', [AssetsController::class, 'orphans']);
    $app->put('/api/assets/upload', [AssetsController::class, 'upload']);
    $app->put('/api/assets/replace', [AssetsController::class, 'replaceAt']);
    $app->get("/api/assets/$asset", [AssetsController::class, 'get']);
    $app->put("/api/assets/$asset", [AssetsController::class, 'update']);

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
        // mail from the shared mailbox (SMTP) and the replies read back (IMAP) – agents/EMBASSY.md "Automation"
        $embassy->post("/orgs/$id/mail", [EmbassyController::class, 'mail']);
        $embassy->get('/inbox', [EmbassyController::class, 'inbox']);
        $embassy->post('/inbox/sync', [EmbassyController::class, 'syncInbox']);
        $embassy->put("/people/$id", [EmbassyController::class, 'updatePerson']);
        $embassy->delete("/people/$id", [EmbassyController::class, 'deletePerson']);
        $embassy->post('/import', [EmbassyController::class, 'import']);
        // pace + the weekly portion (owner 2026-10-05), reminders from the host's cron (no session: a key)
        $embassy->put('/pace', [EmbassyController::class, 'setPace']);
        $embassy->post('/pace/checkin', [EmbassyController::class, 'checkin']);
        $embassy->get('/portion', [EmbassyController::class, 'portion']);
        $embassy->get('/team/pace', [EmbassyController::class, 'teamPace']);
        $embassy->map(['GET', 'POST'], '/cron', [EmbassyController::class, 'cron']);
    });

    // ── Admin API: Session + RequireRole(admin) (docs/API.md "Admin API") ──────────────────────────────
    $app->group('/api/admin', function (RouteCollectorProxy $admin): void {
        $admin->get('/dashboard', [SystemController::class, 'dashboard']);
        $admin->get('/events', [SystemController::class, 'events']);
        // the Ask panel's AI usage per day / month, creator, service, model (docs/API.md "Agent usage")
        $admin->get('/agent-usage', [AgentUsageController::class, 'overview']);
        $admin->get('/catalog', [SystemController::class, 'catalog']);
        // when picked-up placed items reappear, per item (an update of the item's `respawn` in catalog_items)
        // the catalog tables (agents/CATALOG.md): per kind list / get / put / delete / reorder, history + undo
        $kind = '{kind:items|recipes|albums|products|editions|featured|permissions|code-types|toons|meta}';
        $entry = '{id:[A-Za-z0-9][A-Za-z0-9_:.-]{0,190}}';
        $admin->get('/catalog/history', [CatalogAdminController::class, 'history']);
        $admin->post('/catalog/history/{id:[0-9]{1,18}}/undo', [CatalogAdminController::class, 'undo']);
        $admin->get("/catalog/$kind", [CatalogAdminController::class, 'list']);
        $admin->post("/catalog/$kind/reorder", [CatalogAdminController::class, 'reorder']);
        $admin->get("/catalog/$kind/$entry", [CatalogAdminController::class, 'get']);
        $admin->put("/catalog/$kind/$entry", [CatalogAdminController::class, 'put']);
        $admin->delete("/catalog/$kind/$entry", [CatalogAdminController::class, 'delete']);
        $admin->get('/keys', [SystemController::class, 'keys']);
        $admin->get('/versions', VersionsController::class);
        // the newest entries of the API's / the qr app's PHP error log (BACKLOG 10.11)
        $admin->get('/server-errors', [ServerLogController::class, 'list']);
        $admin->get('/settings', [SettingsController::class, 'get']);
        $admin->put('/settings', [SettingsController::class, 'put']);
        // the email module (Domain\Mail): templates + previews, the sent-mail log, the admin notification recipients
        $admin->get('/emails/templates', [EmailsController::class, 'templates']);
        $admin->get('/emails/templates/{id}/preview', [EmailsController::class, 'preview']);
        $admin->get('/emails/log', [EmailsController::class, 'log']);
        $admin->get('/emails/recipients', [EmailsController::class, 'recipients']);
        $admin->put('/emails/recipients', [EmailsController::class, 'saveRecipients']);
        // newsletters (Domain\Newsletter, agents/NEWSLETTER.md): contacts + import, suppressions, newsletters, a batch now
        $newsletter = '{id:[0-9]{1,18}}';
        $admin->get('/newsletter/status', [AdminNewsletterController::class, 'status']);
        $admin->get('/newsletter/audiences', [AdminNewsletterController::class, 'audiences']);
        $admin->get('/newsletter/contacts', [AdminNewsletterController::class, 'contacts']);
        $admin->post('/newsletter/contacts/import', [AdminNewsletterController::class, 'import']);
        $admin->put('/newsletter/contacts', [AdminNewsletterController::class, 'updateContacts']);
        $admin->post('/newsletter/contacts/delete', [AdminNewsletterController::class, 'deleteContacts']);
        $admin->put("/newsletter/contacts/$newsletter", [AdminNewsletterController::class, 'updateContact']);
        $admin->get('/newsletter/suppressions', [AdminNewsletterController::class, 'suppressions']);
        $admin->post('/newsletter/suppressions', [AdminNewsletterController::class, 'suppress']);
        $admin->post('/newsletter/suppressions/remove', [AdminNewsletterController::class, 'unsuppress']);
        $admin->get('/newsletter/campaigns', [AdminNewsletterController::class, 'campaigns']);
        $admin->post('/newsletter/campaigns', [AdminNewsletterController::class, 'create']);
        $admin->get("/newsletter/campaigns/$newsletter", [AdminNewsletterController::class, 'get']);
        $admin->put("/newsletter/campaigns/$newsletter", [AdminNewsletterController::class, 'update']);
        $admin->delete("/newsletter/campaigns/$newsletter", [AdminNewsletterController::class, 'delete']);
        $admin->post("/newsletter/campaigns/$newsletter/preview", [AdminNewsletterController::class, 'preview']);
        $admin->post("/newsletter/campaigns/$newsletter/test", [AdminNewsletterController::class, 'test']);
        $admin->post("/newsletter/campaigns/$newsletter/start", [AdminNewsletterController::class, 'start']);
        $admin->post("/newsletter/campaigns/$newsletter/pause", [AdminNewsletterController::class, 'pause']);
        $admin->post("/newsletter/campaigns/$newsletter/resume", [AdminNewsletterController::class, 'resume']);
        $admin->get("/newsletter/campaigns/$newsletter/recipients", [AdminNewsletterController::class, 'recipients']);
        $admin->post('/newsletter/send', [AdminNewsletterController::class, 'send']);
        // the editions on the landing page (table catalog_featured; GET /api/products/featured/ reads it)
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
        $admin->get('/review-rules', [UsersController::class, 'reviewRules']);
        $admin->put('/users/{id}/review-rules/{toon}/{rule}', [UsersController::class, 'addReviewRule']);
        $admin->delete('/users/{id}/review-rules/{toon}/{rule}', [UsersController::class, 'removeReviewRule']);
        $admin->put('/users/{id}/characters/{character}', [UsersController::class, 'unlockCharacter']);
        $admin->delete('/users/{id}/characters/{character}', [UsersController::class, 'lockCharacter']);
        $admin->delete('/users/{id}/characters/{character}/looks/{look}', [UsersController::class, 'lockLook']);
        $admin->get('/roles', [UsersController::class, 'roles']);

        $admin->get('/content', [AdminContentController::class, 'index']);

        // the asset manager's storage operations (agents/ASSETS.md "Operations")
        $asset = '{id:[0-9a-f-]{36}}';
        $admin->get('/assets/storage', [AssetsController::class, 'storage']);
        $admin->get('/assets/jobs', [AssetsController::class, 'jobs']);
        $admin->post('/assets/scan', [AssetsController::class, 'scan']);
        $admin->post('/assets/relink', [AssetsController::class, 'relink']);
        $admin->get('/assets/unused', [AssetsController::class, 'unused']);
        $admin->post('/assets/cleanup', [AssetsController::class, 'cleanup']);
        $admin->put("/assets/$asset/file", [AssetsController::class, 'replace']);
        $admin->post("/assets/$asset/rename", [AssetsController::class, 'rename']);
        $admin->get("/assets/$asset/delete-check", [AssetsController::class, 'deleteCheck']);
        $admin->delete("/assets/$asset", [AssetsController::class, 'delete']);
        $admin->post("/assets/$asset/jobs", [AssetsController::class, 'enqueue']);
    })->add(new RequireRole($c->get(Roles::class), Roles::ADMIN));

    // Everything else below /api/ is unknown (ErrorHandler → 404 not_found "Unknown endpoint.")
    $app->any('/api[/{rest:.*}]', NotFoundController::class);

    // ── LEGACY URLs, PUBLIC: the verify page (qr app, proxied) and printed certificate links – what is left of the
    //    retired generate/ SPA (removed 2026-10-09; the admin app does the rest) ───────────────────────────────────────
    $app->any('/generate/get-signature.php', [SignatureController::class, 'signature']);
    $app->get('/generate/files/signatures/{name:SIGNATURE-[A-Za-z0-9_-]{1,32}\.json}', [SignatureController::class, 'signatureFile']); // verify page fallback

    // Every other path: 404 (plain text). The QR landing page /?c= and /verify.html live in the qr app (qr/), which
    // proxies /api/validate/, /api/keys/keys.php, the verify page's get-signature.php / signature files (+ /api/*) here.
    $app->any('/[{path:.*}]', PageNotFoundController::class);
};
