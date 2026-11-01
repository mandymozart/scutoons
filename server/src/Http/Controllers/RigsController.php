<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Rigs\MemberRigs;
use Scutoons\Domain\Rigs\RigValidator;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Support\Json as JsonEncoding;

/**
 * /api/rigs – the member's own presets and rig variants (#270, Domain\Rigs\MemberRigs; agents/research/INSTRUMENTS.md
 * §2, §4–6; docs/API.md "Rigs"). A session, or the paired deck's token (#180) with the `rigs` scope its member granted
 * at approval (DeckTokenScope) – "save as preset" from the deck's knobs. Every row is the member's own.
 *   GET    /rigs/presets        → { status, items: [Preset] } – the member's derivatives, oldest first
 *   POST   /rigs/presets        { parent, values?, name?, tags?, mapping? } → 201 { status, preset } – a derivative
 *                               holding only the values that differ from the parent's folded lineage
 *                               · 403 not_derivable { hold } · 422 invalid_preset { problems }
 *   DELETE /rigs/presets/{id}   → { status } · 404 not_found · 409 in_use { uses }
 *   GET    /rigs/{subject}      → { status, subject, rig } – the member's variant (null: none) · 404 not_found (no factory rig)
 *   PUT    /rigs/{subject}      a Rig (subject filled in / must match) → { status, subject, rig } · 404 · 422 invalid_rig
 *   DELETE /rigs/{subject}      → { status }
 * Preset and rig bodies are deck-space's serde forms (floats stay floats: JSON_PRESERVE_ZERO_FRACTION).
 */
final class RigsController
{
    public function __construct(private readonly MemberRigs $members)
    {
    }

    public function presets(Request $request, Response $response): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        return self::json($response, 200, ['status' => 'success', 'items' => $this->members->presets((string) $user['id'])]);
    }

    public function derive(Request $request, Response $response): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        $body = JsonBody::object($request) ?? throw new ApiError(400, 'invalid_json', 'The request body is not a JSON object.');
        $preset = $this->members->derive((string) $user['id'], $body);
        return self::json($response, 201, ['status' => 'success', 'preset' => $preset]);
    }

    /** @param array{id: string} $args */
    public function deletePreset(Request $request, Response $response, array $args): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        $this->members->deletePreset((string) $user['id'], $args['id']);
        return self::json($response, 200, ['status' => 'success']);
    }

    /** @param array{subject: string} $args */
    public function rig(Request $request, Response $response, array $args): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        $subject = self::subject($args);
        $this->members->factoryRig($subject);
        return self::json($response, 200, ['status' => 'success', 'subject' => $subject, 'rig' => $this->members->rig((string) $user['id'], $subject)]);
    }

    /** @param array{subject: string} $args */
    public function saveRig(Request $request, Response $response, array $args): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        $subject = self::subject($args);
        $body = JsonBody::object($request) ?? throw new ApiError(400, 'invalid_json', 'The request body is not a JSON object.');
        if (isset($body->subject) && $body->subject !== $subject) {
            throw new ApiError(400, 'subject_mismatch', "The body's subject must be $subject (or left out).");
        }
        $this->members->saveRig((string) $user['id'], $subject, $body);
        return self::json($response, 200, ['status' => 'success', 'subject' => $subject, 'rig' => $this->members->rig((string) $user['id'], $subject)]);
    }

    /** @param array{subject: string} $args */
    public function deleteRig(Request $request, Response $response, array $args): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        $this->members->deleteRig((string) $user['id'], self::subject($args));
        return self::json($response, 200, ['status' => 'success']);
    }

    /** @param array{subject: string} $args */
    private static function subject(array $args): string
    {
        $subject = rawurldecode($args['subject']);
        if (!preg_match(RigValidator::SUBJECT, $subject)) throw new ApiError(400, 'invalid_subject', 'A rig\'s subject is character.<id>[.<persona>] or item.<id>.');
        return $subject;
    }

    /** the serde forms keep their floats (`1.0` stays `1.0` – a node param stays a float in TOML) */
    private static function json(Response $response, int $status, mixed $data): Response
    {
        return Json::response($response, $status, $data, [], JsonEncoding::FLAGS | JSON_PRESERVE_ZERO_FRACTION);
    }
}
