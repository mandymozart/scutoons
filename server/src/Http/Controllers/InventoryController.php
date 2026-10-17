<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Codes\CodeRules;
use Scutoons\Domain\Catalog\Catalog;
use Scutoons\Domain\Codes\CodeService;
use Scutoons\Domain\Homes\Homes;
use Scutoons\Domain\Inventory\Containers;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Domain\Ownership\Gifts;
use Scutoons\Domain\Ownership\OwnershipTransfers;
use Scutoons\Domain\Reader\ReaderState;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 * Signed-in inventory actions (Bearer):
 *   POST /api/inventory/claim/   { code, item_id?, variant? } → { status, claimed: "new"|"already_yours", code, inventory }
 *   POST /api/inventory/craft/   { recipe_id, use?: { <item id>: <entry id> }, workshop?: { home } | { visit } } → { status, recipe_id, inventory }
 *                                (a recipe `at: "workshop"` needs a workbench there – 409 needs_workshop; research/CAVE.md D4)
 *   POST /api/inventory/move/    { entry_id, into: <container entry id> | null, overfill?, qty? } → { status, inventory } (containers, research/SCORES.md)
 *   POST /api/inventory/stack/   { entry_id, onto } → { status, inventory } (two of the same kind onto one field, research/PACKING.md §6)
 *   POST /api/inventory/grant/   { item_id, source, ref? } → { status, granted, inventory }
 *   POST /api/inventory/take/    { item_id, qty?, ref? } → { status, taken, reason: null|too_few|already_taken, inventory } (a story takes – BACKLOG 10.38)
 *   POST /api/inventory/import/  { items: [{ item_id, qty, variant?, local_id?, in? }] } → { status, imported, skipped, inventory }
 *   POST /api/inventory/use/     { item_id } → { status, item_id, stats, effects, inventory } (a substance, ITEMS.md "Substances")
 *   POST /api/effects/end/       { kind } → { status, ended, effects } ("Effekt beenden")
 *   POST /api/inventory/gift/    { entry_id | item_id (+ variant), qty?, to: email } → { status, gift, inventory } (ITEMS.md "Ownership, claims, gifting")
 *   POST /api/inventory/received/seen/  { ids? } → { status, seen } (the notices of GET /me/ `received`)
 */
final class InventoryController
{
    public function __construct(
        private readonly InventoryService $inventory,
        private readonly CodeService $codes,
        private readonly ReaderState $reader,
        private readonly Gifts $gifts,
        private readonly OwnershipTransfers $transfers,
        private readonly Containers $containers,
        private readonly Catalog $catalog,
        private readonly Homes $homes,
    ) {
    }

    public function gift(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $user = CurrentSession::of($request)->requireUser();
        $gift = $this->gifts->gift($user, $body);
        return Json::response($response, 200, ['status' => 'success', 'gift' => $gift, 'inventory' => $this->inventory->entries((string) $user['id'])]);
    }

    public function seen(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $user = CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, ['status' => 'success', 'seen' => $this->transfers->markSeen((string) $user['id'], $body['ids'] ?? null)]);
    }

    public function use(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $user = CurrentSession::of($request)->requireUser();
        $id = (string) $user['id'];
        $item = $this->reader->use($id, $body);
        return Json::response($response, 200, ['status' => 'success', 'item_id' => $item['id'], 'stats' => (object) $this->reader->stats($id),
            'effects' => $this->reader->effects($id), 'inventory' => $this->inventory->entries($id)]);
    }

    public function endEffect(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $user = CurrentSession::of($request)->requireUser();
        $id = (string) $user['id'];
        $kind = is_string($body['kind'] ?? null) ? $body['kind'] : throw new ApiError(400, 'invalid_kind', 'kind is required.');
        $ended = $this->reader->end($id, $kind);
        return Json::response($response, 200, ['status' => 'success', 'ended' => $ended, 'effects' => $this->reader->effects($id)]);
    }

    public function claim(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $user = CurrentSession::of($request)->requireUser();
        $code = CodeRules::normalize($body['code'] ?? null) ?? throw new ApiError(400, 'invalid_code', 'Code missing or malformed.');
        $itemId = is_string($body['item_id'] ?? null) ? $body['item_id'] : null;
        $variant = is_string($body['variant'] ?? null) ? $body['variant'] : null;
        $new = $this->codes->claim($user, $code, $itemId, $variant);
        return Json::response($response, 200, ['status' => 'success', 'claimed' => $new ? 'new' : 'already_yours', 'code' => $code,
            'inventory' => $this->inventory->entries((string) $user['id'])]);
    }

    public function craft(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $user = CurrentSession::of($request)->requireUser();
        $wanted = is_string($body['recipe_id'] ?? null) ? $this->catalog->recipe($body['recipe_id']) : null;
        if (($wanted['at'] ?? null) === 'workshop' && !$this->homes->workshopAt((string) $user['id'], $body['workshop'] ?? null)) {
            throw new ApiError(409, 'needs_workshop', 'This can only be made at a workbench.');
        }
        $recipe = $this->inventory->craft((string) $user['id'], $body['recipe_id'] ?? null, $body['use'] ?? null);
        return Json::response($response, 200, ['status' => 'success', 'recipe_id' => $recipe['id'], 'inventory' => $this->inventory->entries((string) $user['id'])]);
    }

    /** Into a container or out (research/SCORES.md §2): 409 overfill_warning until the reader agrees (`overfill: true`) */
    public function move(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $user = CurrentSession::of($request)->requireUser();
        $this->containers->move((string) $user['id'], $body);
        return Json::response($response, 200, ['status' => 'success', 'inventory' => $this->inventory->entries((string) $user['id'])]);
    }

    public function stack(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $user = CurrentSession::of($request)->requireUser();
        $this->containers->stack((string) $user['id'], $body);
        return Json::response($response, 200, ['status' => 'success', 'inventory' => $this->inventory->entries((string) $user['id'])]);
    }

    public function grant(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $user = CurrentSession::of($request)->requireUser();
        $granted = $this->inventory->grant((string) $user['id'], $body);
        return Json::response($response, 200, ['status' => 'success', 'granted' => $granted, 'inventory' => $this->inventory->entries((string) $user['id'])]);
    }

    public function take(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $user = CurrentSession::of($request)->requireUser();
        $result = $this->inventory->take((string) $user['id'], $body);
        return Json::response($response, 200, ['status' => 'success'] + $result + ['inventory' => $this->inventory->entries((string) $user['id'])]);
    }

    public function import(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        $user = CurrentSession::of($request)->requireUser();
        $result = $this->inventory->import((string) $user['id'], $body);
        return Json::response($response, 200, ['status' => 'success'] + $result + ['inventory' => $this->inventory->entries((string) $user['id'])]);
    }
}
