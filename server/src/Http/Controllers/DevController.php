<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Dev\Migrations;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Infrastructure\Config\Settings;

/**
 * The migrations through the API (BACKLOG 10.61, Domain\Dev\Migrations).
 *
 * /api/dev – role developer (a session or the developer's AI connection – their agents over MCP):
 *   GET  /dev/migrations                   → { status, database, base_schema, pending, files: [{ name, applied_at, sha256 }], runs }
 *   POST /dev/migrations/plan              → { status, run } (201) – the dry run, stored; 409 busy · 409 no_schema
 *   GET  /dev/migrations/runs/{id}         → { status, run }
 *   POST /dev/migrations/runs/{id}/apply   → { status, run } (run.status applied | failed) · 409 not_planned / plan_stale / busy
 *   POST /dev/migrations/run               → { status, run } – plan + apply in one go; run null = nothing pending
 *
 * /api/deploy/migrate – no session: the header `X-Migrate-Key: <MIGRATE_KEY>` (the deploy, right after the API upload);
 *   POST → { status, run } like /dev/migrations/run; HTTP 500 when the run failed (the deploy job fails). MIGRATE_KEY
 *   empty = 404.
 */
final class DevController
{
    public const KEY_HEADER = 'X-Migrate-Key';

    public function __construct(private readonly Migrations $migrations, private readonly Settings $settings)
    {
    }

    public function status(Request $request, Response $response): Response
    {
        return Json::response($response, 200, ['status' => 'success'] + $this->migrations->status());
    }

    public function plan(Request $request, Response $response): Response
    {
        return Json::response($response, 201, ['status' => 'success', 'run' => $this->migrations->plan($this->user($request), self::via($request))]);
    }

    /** @param array{id: string} $args */
    public function get(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success', 'run' => $this->migrations->get($args['id'])]);
    }

    /** @param array{id: string} $args */
    public function apply(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success', 'run' => $this->migrations->apply($args['id'], $this->user($request))]);
    }

    public function run(Request $request, Response $response): Response
    {
        return Json::response($response, 200, ['status' => 'success', 'run' => $this->migrations->run($this->user($request), self::via($request))]);
    }

    /** The deploy's call: the key, compared in constant time */
    public function deploy(Request $request, Response $response): Response
    {
        $key = $this->settings->get('MIGRATE_KEY');
        if ($key === '') throw new ApiError(404, 'not_found', 'Not found.');
        $given = $request->getHeaderLine(self::KEY_HEADER);
        if ($given === '' || !hash_equals($key, $given)) throw new ApiError(403, 'forbidden', 'Wrong key.');
        $run = $this->migrations->run(null, 'deploy');
        return Json::response($response, ($run['status'] ?? 'applied') === 'failed' ? 500 : 200, ['status' => 'success', 'run' => $run]);
    }

    /** session | ai:<the AI connection's name> */
    private static function via(Request $request): string
    {
        $token = CurrentSession::of($request)->apiToken();
        return $token !== null ? 'ai:' . $token['name'] : 'session';
    }

    /** @return array<string, mixed> */
    private function user(Request $request): array
    {
        $user = $request->getAttribute('user');
        return is_array($user) ? $user : [];
    }
}
