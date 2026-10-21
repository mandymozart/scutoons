<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/** GET /api/mcp/download and /api/mcp/version – the member's MCP bundle shipped with the API (issue 76) */
final class McpApiTest extends TestCase
{
    /** @return array<string, string> */
    protected function settings(): array
    {
        return ['MCP_PATH' => $this->dir . '/mcp'] + parent::settings();
    }

    /** @param array<string, string> $headers */
    private function request(string $uri, array $headers = []): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest('GET', "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        return Kernel::app($this->container)->handle($request);
    }

    /** @return array<mixed> */
    private static function json(ResponseInterface $response): array
    {
        return (array) json_decode((string) $response->getBody(), true);
    }

    private function build(): string
    {
        $dir = $this->dir . '/mcp';
        mkdir($dir);
        $code = "// SCUTOONS MCP server 0.4.0\nconsole.log('hi');\n";
        file_put_contents("$dir/scutoons-mcp.mjs", $code);
        file_put_contents("$dir/scutoons.mcpb", "PK\x05\x06" . str_repeat("\0", 18));
        file_put_contents("$dir/scutoons-figma-plugin.zip", "PK\x05\x06" . str_repeat("\1", 18));
        file_put_contents("$dir/scutoons-mcp.json", (string) json_encode([
            'version' => '0.4.0', 'git_sha' => 'abc123', 'sha256' => hash('sha256', $code),
            'sha256_mcpb' => hash_file('sha256', "$dir/scutoons.mcpb"),
            'sha256_figma_plugin' => hash_file('sha256', "$dir/scutoons-figma-plugin.zip"), 'figma_plugin_version' => '2026-10-20.4',
            'built_at' => '2026-10-20T10:00:00Z',
        ]));
        return $code;
    }

    public function testNotBuiltIs404(): void
    {
        self::assertSame([404, 'not_built'], [($r = $this->request('/api/mcp/version'))->getStatusCode(), self::json($r)['error']]);
        self::assertSame(404, $this->request('/api/mcp/download')->getStatusCode());
    }

    public function testVersion(): void
    {
        $code = $this->build();
        $r = $this->request('/api/mcp/version/');
        self::assertSame(200, $r->getStatusCode());
        $body = self::json($r);
        self::assertSame(['0.4.0', 'abc123', hash('sha256', $code)], [$body['version'], $body['git_sha'], $body['sha256']]);
        self::assertSame([hash_file('sha256', $this->dir . '/mcp/scutoons-figma-plugin.zip'), '2026-10-20.4'], [$body['sha256_figma_plugin'], $body['figma_plugin_version']]);
        self::assertSame('public, max-age=60', $r->getHeaderLine('Cache-Control'));
    }

    public function testDownloadWithEtag(): void
    {
        $code = $this->build();
        $r = $this->request('/api/mcp/download');
        self::assertSame(200, $r->getStatusCode());
        self::assertSame($code, (string) $r->getBody());
        self::assertSame('application/javascript; charset=utf-8', $r->getHeaderLine('Content-Type'));
        self::assertSame('attachment; filename="scutoons-mcp.mjs"', $r->getHeaderLine('Content-Disposition'));
        self::assertSame('public, max-age=300', $r->getHeaderLine('Cache-Control'));
        $etag = $r->getHeaderLine('ETag');
        self::assertSame('"m' . substr(hash('sha256', $code), 0, 32) . '"', $etag);

        $again = $this->request('/api/mcp/download', ['If-None-Match' => $etag]);
        self::assertSame([304, ''], [$again->getStatusCode(), (string) $again->getBody()]);

        $mcpb = $this->request('/api/mcp/download?file=mcpb');
        self::assertSame(['application/zip', 'attachment; filename="scutoons.mcpb"'], [$mcpb->getHeaderLine('Content-Type'), $mcpb->getHeaderLine('Content-Disposition')]);
        self::assertNotSame($etag, $mcpb->getHeaderLine('ETag'));
        self::assertSame(400, $this->request('/api/mcp/download?file=../config.php')->getStatusCode());
    }

    /** the SCUTOONS Bridge plugin for Figma desktop (#127): a zip with its own ETag; an older build without it → 404 */
    public function testFigmaPluginDownload(): void
    {
        $this->build();
        $zip = (string) file_get_contents($this->dir . '/mcp/scutoons-figma-plugin.zip');
        $r = $this->request('/api/mcp/download?file=figma-plugin');
        self::assertSame(200, $r->getStatusCode());
        self::assertSame($zip, (string) $r->getBody());
        self::assertSame(['application/zip', 'attachment; filename="scutoons-figma-plugin.zip"'], [$r->getHeaderLine('Content-Type'), $r->getHeaderLine('Content-Disposition')]);
        self::assertSame('"m' . substr(hash('sha256', $zip), 0, 32) . '"', $r->getHeaderLine('ETag'));

        unlink($this->dir . '/mcp/scutoons-figma-plugin.zip');
        self::assertSame([404, 'not_built'], [($gone = $this->request('/api/mcp/download?file=figma-plugin'))->getStatusCode(), self::json($gone)['error']]);
    }
}
