<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\CatalogStore;
use Scutoons\Domain\Rigs\MemberRigs;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use stdClass;

/**
 * A member's presets and rig variants (#270, agents/research/INSTRUMENTS.md §5–6, docs/API.md "Rigs"): a signed-in
 * member – a session, or their paired deck's token (`Bearer scd_…`, DeckTokenScope lets these writes through: a deck
 * saves what its knobs made). Members' rows are private: every call reads and writes the caller's own only.
 *   GET    /api/rigs/presets              → { status, items: [Preset] }
 *   POST   /api/rigs/presets              { parent, values?, name?, tags?, unlink?, link?, mapping? } → 201 { status, preset }
 *   DELETE /api/rigs/presets/{m:…}        → { status } · 404 · 409 in_use
 *   GET    /api/rigs/{subject}            → { status, subject, rig: Rig|null } · 404 (no factory rig)
 *   PUT    /api/rigs/{subject}            Rig → { status, subject, rig } · 422 invalid_rig
 *   DELETE /api/rigs/{subject}            → { status }
 */
final class RigsController
{
    public function __construct(private readonly MemberRigs $rigs, private readonly CatalogStore $catalog)
    {
    }

    public function presets(Request $request, Response $response): Response
    {
        $user = self::user($request);
        return Json::response($response, 200, ['status' => 'success', 'items' => $this->rigs->presets($user)]);
    }

    public function savePreset(Request $request, Response $response): Response
    {
        $user = self::user($request);
        $preset = $this->rigs->savePreset($user, self::body($request));
        return Json::response($response, 201, ['status' => 'success', 'preset' => $preset]);
    }

    /** @param array<string, string> $args */
    public function deletePreset(Request $request, Response $response, array $args): Response
    {
        $this->rigs->deletePreset(self::user($request), $args['id']);
        return Json::response($response, 200, ['status' => 'success']);
    }

    /** @param array<string, string> $args */
    public function rig(Request $request, Response $response, array $args): Response
    {
        $user = self::user($request);
        $subject = $this->subject($args);
        return Json::response($response, 200, ['status' => 'success', 'subject' => $subject, 'rig' => $this->rigs->rig($user, $subject)]);
    }

    /** @param array<string, string> $args */
    public function saveRig(Request $request, Response $response, array $args): Response
    {
        $user = self::user($request);
        $subject = $this->subject($args);
        $this->rigs->saveRig($user, $subject, self::body($request));
        return Json::response($response, 200, ['status' => 'success', 'subject' => $subject, 'rig' => $this->rigs->rig($user, $subject)]);
    }

    /** @param array<string, string> $args */
    public function deleteRig(Request $request, Response $response, array $args): Response
    {
        $user = self::user($request);
        $this->rigs->deleteRig($user, $this->subject($args));
        return Json::response($response, 200, ['status' => 'success']);
    }

    private static function user(Request $request): string
    {
        return (string) CurrentSession::of($request)->requireUser()['id'];
    }

    /** the body decoded with objects (`{}` ≠ `[]`, as RigValidator reads it) */
    private static function body(Request $request): stdClass
    {
        return JsonBody::object($request) ?? throw new ApiError(400, 'invalid_json', 'The request body is not a JSON object.');
    }

    /** @param array<string, string> $args */
    private function subject(array $args): string
    {
        $subject = $args['subject'];
        if ($this->catalog->get('rigs', $subject) === null) throw new ApiError(404, 'not_found', "No rig $subject.");
        return $subject;
    }
}
