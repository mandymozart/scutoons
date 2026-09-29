<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers\Admin;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Admin\Versions;
use Scutoons\Http\Json;

/**
 * GET /api/admin/versions (role admin) → { status, checked_at, api: { app, version, git_sha, built_at },
 * apps: [{ app, url, ok, http_status, version?, git_sha?, built_at?, shared?, error? }] } – the deployed version.json
 * of every app (VERSION_URLS) next to the API's own, for the admin dashboard's out-of-sync check.
 */
final class VersionsController
{
    public function __construct(private readonly Versions $versions)
    {
    }

    public function __invoke(Request $request, Response $response): Response
    {
        return Json::response($response, 200, $this->versions->all());
    }
}
