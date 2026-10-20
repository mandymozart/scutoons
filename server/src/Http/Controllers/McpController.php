<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Http\Download;
use Scutoons\Http\Json;
use Scutoons\Infrastructure\Config\Settings;

/**
 * The member's SCUTOONS MCP server, shipped with the API (agents/MCP.md §3a, issue 76): the api deploy builds it
 * (tools/mcp/scripts/build.mjs) into `MCP_PATH` – scutoons-mcp.mjs, scutoons.mcpb and scutoons-mcp.json
 * { version, git_sha, sha256, sha256_mcpb, built_at }. Locally `npm run build:mcp` writes the same folder.
 *
 * GET /api/mcp/download            → scutoons-mcp.mjs (Claude Code; short link scutoons.com/mcp)
 * GET /api/mcp/download?file=mcpb  → scutoons.mcpb (the Claude Desktop extension; scutoons.com/mcp.mcpb)
 * GET /api/mcp/version             → { version, git_sha, sha256, sha256_mcpb } – the local MCP checks it on start
 * Not built → 404 not_built.
 */
final class McpController
{
    private const FILES = [
        'mjs' => ['scutoons-mcp.mjs', 'application/javascript; charset=utf-8', 'sha256'],
        'mcpb' => ['scutoons.mcpb', 'application/zip', 'sha256_mcpb'],
    ];

    public function __construct(private readonly Settings $settings)
    {
    }

    /** @return array<string, mixed>|null */
    private function manifest(): ?array
    {
        $dir = $this->settings->path('MCP_PATH');
        $data = json_decode((string) @file_get_contents($dir . '/scutoons-mcp.json'), true);
        return is_array($data) && is_string($data['version'] ?? null) ? $data : null;
    }

    public function version(Request $request, Response $response): Response
    {
        $manifest = $this->manifest();
        if ($manifest === null) return Json::response($response, 404, ['error' => 'not_built']);
        return Json::response($response, 200, [
            'version' => $manifest['version'],
            'git_sha' => (string) ($manifest['git_sha'] ?? ''),
            'sha256' => (string) ($manifest['sha256'] ?? ''),
            'sha256_mcpb' => (string) ($manifest['sha256_mcpb'] ?? ''),
        ], ['Cache-Control' => 'public, max-age=60', 'Access-Control-Allow-Origin' => '*']);
    }

    public function download(Request $request, Response $response): Response
    {
        $kind = (string) ($request->getQueryParams()['file'] ?? 'mjs');
        if (!isset(self::FILES[$kind])) return Json::response($response, 400, ['error' => 'invalid_file']);
        [$name, $type, $hashKey] = self::FILES[$kind];
        $manifest = $this->manifest();
        $path = $this->settings->path('MCP_PATH') . '/' . $name;
        if ($manifest === null || !is_file($path)) return Json::response($response, 404, ['error' => 'not_built']);
        $hash = is_string($manifest[$hashKey] ?? null) && $manifest[$hashKey] !== '' ? $manifest[$hashKey] : (string) hash_file('sha256', $path);
        $etag = '"m' . substr($hash, 0, 32) . '"';
        $response = $response->withHeader('ETag', $etag);
        if (trim($request->getHeaderLine('If-None-Match')) === $etag) {
            return $response->withStatus(304)->withHeader('Cache-Control', 'public, max-age=300');
        }
        return Download::file($response, $path, $type, $name)
            ->withHeader('Cache-Control', 'public, max-age=300')
            ->withHeader('X-Content-Type-Options', 'nosniff');
    }
}
