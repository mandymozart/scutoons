<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Http\Json;
use Scutoons\Infrastructure\Database\Database;

/** GET /api/health/ → { status: "ok", db: "ok"|"error" } */
final class HealthController
{
    public function __construct(private readonly Database $db)
    {
    }

    public function __invoke(Request $request, Response $response): Response
    {
        return Json::response($response, 200, ['status' => 'ok', 'db' => $this->db->value('SELECT 1') == 1 ? 'ok' : 'error']);
    }
}
