<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Http\Json;
use Scutoons\Infrastructure\Config\Version;
use Scutoons\Infrastructure\Database\Database;

/** GET /api/health/ → { status: "ok", db: "ok"|"error", version, git_sha, built_at } */
final class HealthController
{
    public function __construct(private readonly Database $db, private readonly Version $version)
    {
    }

    public function __invoke(Request $request, Response $response): Response
    {
        $version = $this->version->info();
        return Json::response($response, 200, [
            'status' => 'ok',
            'db' => $this->db->value('SELECT 1') == 1 ? 'ok' : 'error',
            'version' => $version['version'],
            'git_sha' => $version['git_sha'],
            'built_at' => $version['built_at'],
        ]);
    }
}
