<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\ApiTokens;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Domain\Settings\RuntimeSettings;
use Scutoons\Domain\Storyboard\EuroRates;
use Scutoons\Domain\Storyboard\Generators\GeneratorRegistry;
use Scutoons\Domain\Storyboard\Generators\HttpProvider;
use Scutoons\Domain\Storyboard\Mp4;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\Domain\FakeProviders;
use Scutoons\Tests\Domain\StoryboardTest;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * Paid provider jobs (#121, docs/API.md "Storyboard" → "Provider jobs") against a stand-in provider (FakeProviders –
 * never a real paid call): the generators list, the budget gate (no budget / within / over / reserved counted), start →
 * status → success → cost booked, failures, cancel, the result passed through, the signed reference links, the euro rate.
 */
final class StoryboardJobsApiTest extends TestCase
{
    private FakeProviders $fake;

    protected function setUp(): void
    {
        parent::setUp();
        $this->fake = new FakeProviders();
    }

    protected function settings(): array
    {
        return [
            'STORYBOARD_WORK_PATH' => $this->dir . '/work', 'API_URL' => 'https://api.test',
            'LUMA_API_KEY' => FakeProviders::LUMA_KEY, 'LUMA_API_URL' => 'https://luma.test',
            'RUNWAY_API_KEY' => FakeProviders::RUNWAY_KEY, 'RUNWAY_API_URL' => 'https://runway.test/',
            'ECB_RATES_URL' => '',
        ] + parent::settings();
    }

    /** @param array<string, string> $headers */
    private function request(string $method, string $uri, array $headers = [], mixed $body = null, ?string $raw = null): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        if ($body !== null || $raw !== null) {
            $request->getBody()->write($raw ?? (string) json_encode($body));
            $request->getBody()->rewind();
            if ($raw === null) $request = $request->withHeader('Content-Type', 'application/json');
        }
        $container = Kernel::container(Settings::fixed(self::root(), $this->settings()));
        $container->set(Clock::class, $this->clock);
        $container->get(GeneratorRegistry::class)->useHttp($this->fake);
        $container->get(EuroRates::class)->useHttp($this->fake);
        return Kernel::app($container)->handle($request);
    }

    /** @return array<string, mixed> */
    private static function json(ResponseInterface $response): array
    {
        return (array) json_decode((string) $response->getBody(), true);
    }

    /**
     * @param list<string> $roles
     * @return array{0: array<string, string>, 1: array<string, mixed>}
     */
    private function auth(array $roles, string $email = ''): array
    {
        $user = $this->user($email);
        foreach ($roles as $role) $this->get(Roles::class)->grant((string) $user['id'], $role);
        return [['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])], $user];
    }

    private function budget(?int $euro): void
    {
        $this->get(RuntimeSettings::class)->update([RuntimeSettings::GENERATION_BUDGET => $euro], 'admin:test');
    }

    private const LUMA = ['toon' => 'catpope', 'chapter' => 3, 'shot' => 'sb-25-1', 'generator' => 'luma-image', 'model' => 'uni-1', 'prompt' => 'PLF im Schnee', 'aspect' => '9:16'];

    /**
     * @param array<string, mixed> $extra
     * @return array<string, mixed>
     */
    private static function runway(string $model = 'gen4_turbo', int $duration = 5, array $extra = []): array
    {
        return $extra + ['toon' => 'catpope', 'chapter' => 3, 'shot' => 'sb-25-2', 'generator' => 'runway', 'model' => $model, 'prompt' => 'slow push in', 'aspect' => '9:16',
            'references' => [['role' => 'still', 'url' => 'https://cdn.scutoons.test/catpope/sb-25-2.png']], 'params' => ['duration' => $duration]];
    }

    public function testTheGeneratorsWithoutTheirKeys(): void
    {
        [$creator] = $this->auth([Roles::CREATOR]);
        $r = $this->request('GET', '/api/storyboard/generators', $creator);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        $body = self::json($r);
        $by = array_column($body['generators'], null, 'id');
        self::assertSame([true, 'LUMA_API_KEY', ['uni-1', 'uni-1-max']], [$by['luma-image']['configured'], $by['luma-image']['setting'], array_column($by['luma-image']['models'], 'id')]);
        self::assertSame([true, ['gen4_turbo', 'gen4.5', 'aleph2']], [$by['runway']['configured'], array_column($by['runway']['models'], 'id')]);
        self::assertSame([false, 'GEMINI_API_KEY'], [$by['gemini-image']['configured'], $by['gemini-image']['setting']]);
        self::assertNotNull($by['gemini-image']['unavailable']);
        self::assertStringNotContainsString(FakeProviders::LUMA_KEY, (string) $r->getBody(), 'never a key in an answer');
        self::assertSame(['EUR', 0, 'fallback'], [$body['budget']['currency'], $body['budget']['budget'], $body['budget']['rate']['source']]);
        [$reader] = $this->auth([]);
        self::assertSame(403, $this->request('GET', '/api/storyboard/generators', $reader)->getStatusCode());
        self::assertSame(403, $this->request('POST', '/api/storyboard/generations', $reader, self::LUMA)->getStatusCode());
    }

    public function testNoBudgetRefusesEveryPaidJob(): void
    {
        [$creator] = $this->auth([Roles::CREATOR]);
        $r = $this->request('POST', '/api/storyboard/generations', $creator, self::LUMA);
        self::assertSame([402, 'no_budget', 0, 'EUR'], [$r->getStatusCode(), self::json($r)['error'], self::json($r)['budget'], self::json($r)['currency']]);
        self::assertSame([], $this->fake->calls('luma.test'), 'the provider is never called');
        self::assertSame(0, (int) $this->db->value('SELECT COUNT(*) FROM storyboard_generations'));
    }

    public function testALumaJobFromStartToTheResult(): void
    {
        $this->budget(1);
        [$creator, $user] = $this->auth([Roles::CREATOR], 'creator@scutoons.test');
        $ai = ['Authorization' => 'Bearer ' . $this->get(ApiTokens::class)->create($user, 'Claude on my laptop')['token']];
        $png = StoryboardTest::PNG . str_repeat('x', 100);
        $put = $this->request('PUT', '/api/storyboard/work?toon=catpope&chapter=3&name=sb-25-1-render.png', $ai + ['Content-Type' => 'image/png'], null, $png);
        self::assertSame(201, $put->getStatusCode(), (string) $put->getBody());

        $job = self::LUMA + ['references' => [['role' => 'reference', 'work' => 'sb-25-1-render.png'], ['role' => 'style', 'url' => 'https://cdn.scutoons.test/style/a.webp']], 'params' => ['style' => 'manga']];
        $e = $this->request('POST', '/api/storyboard/generations/estimate', $ai, $job);
        self::assertSame([200, 0.0464, 'USD', '9:16'], [$e->getStatusCode(), self::json($e)['estimate']['amount'], self::json($e)['estimate']['currency'], self::json($e)['estimate']['ratio']], (string) $e->getBody());
        self::assertSame([], $this->fake->calls('luma.test'), 'an estimate starts nothing');

        $r = $this->request('POST', '/api/storyboard/generations', $ai, $job);
        self::assertSame(201, $r->getStatusCode(), (string) $r->getBody());
        $item = self::json($r)['item'];
        self::assertSame(['queued', 'luma-1', 'luma-image', 'image', 0.0464, 'USD', null, 'Claude on my laptop', 'creator@scutoons.test'],
            [$item['status'], $item['job'], $item['generator'], $item['kind'], $item['estimate'], $item['currency'], $item['cost'], $item['via'], $item['author']]);
        self::assertSame([['role' => 'reference', 'work' => 'sb-25-1-render.png'], ['role' => 'style', 'url' => 'https://cdn.scutoons.test/style/a.webp']], $item['inputs'], 'the row keeps the source, never the signed link');
        $sent = $this->fake->lastBody('/v1/generations');
        self::assertSame(['PLF im Schnee', 'uni-1', 'image', '9:16', 'manga', 'png'], [$sent['prompt'], $sent['model'], $sent['type'], $sent['aspect_ratio'], $sent['style'], $sent['output_format']]);
        self::assertSame('https://cdn.scutoons.test/style/a.webp', $sent['image_ref'][1]['url']);

        // the signed link Luma fetches the work file by: no session, that file, 15 minutes
        $link = (string) $sent['image_ref'][0]['url'];
        self::assertStringStartsWith('https://api.test/api/storyboard/ref/catpope/3/sb-25-1-render.png?expires=', $link);
        $path = substr($link, strlen('https://api.test'));
        $g = $this->request('GET', $path);
        self::assertSame([200, $png, 'private, no-store'], [$g->getStatusCode(), (string) $g->getBody(), $g->getHeaderLine('Cache-Control')]);
        self::assertSame(404, $this->request('GET', substr($path, 0, -1) . (str_ends_with($path, 'a') ? 'b' : 'a'))->getStatusCode(), 'a changed signature');
        self::assertSame(404, $this->request('GET', str_replace('sb-25-1-render.png', 'other.png', $path))->getStatusCode(), 'another file');

        // polled when read – at most once per GENERATION_POLL_SECONDS
        $this->fake->luma['luma-1']['state'] = 'processing';
        self::assertSame('running', self::json($this->request('GET', "/api/storyboard/generations/{$item['id']}", $creator))['item']['status']);
        $this->request('GET', "/api/storyboard/generations/{$item['id']}", $creator);
        $this->request('GET', '/api/storyboard/generations?toon=catpope&chapter=3', $creator);
        self::assertSame(['POST /v1/generations', 'GET /v1/generations/luma-1'], $this->fake->calls('luma.test'), 'asked once within the 5 s');
        self::assertSame([0.04, 0.04], [self::json($this->request('GET', '/api/storyboard/budget', $creator))['budget']['reserved'], round(0.0464 / 1.1225, 2)]);

        $this->clock->advance(6);
        $this->fake->luma['luma-1'] = ['id' => 'luma-1', 'state' => 'completed', 'output' => [['type' => 'image', 'url' => 'https://files.test/luma-1.png']]];
        $list = self::json($this->request('GET', '/api/storyboard/generations?toon=catpope&chapter=3', $creator))['items'];
        $done = $list[0];
        self::assertSame(['succeeded', 0.0464, 'https://files.test/luma-1.png', "/api/storyboard/generations/{$item['id']}/result", 1], [$done['status'], $done['cost'], $done['result_url'], $done['result'], $done['progress']]);
        $budget = self::json($this->request('GET', '/api/storyboard/budget', $creator))['budget'];
        self::assertSame([1, 0.04, 0, 0.96], [$budget['budget'], $budget['spent'], $budget['reserved'], $budget['left']]);

        // the result passed through (the provider's CORS); nothing stored
        $res = $this->request('GET', "/api/storyboard/generations/{$item['id']}/result", $creator);
        self::assertSame([200, FakeProviders::PNG, 'image/png'], [$res->getStatusCode(), (string) $res->getBody(), $res->getHeaderLine('Content-Type')]);
        // the MCP saved it into the member's workspace folder and says where
        $hash = hash('sha256', FakeProviders::PNG);
        $saved = $this->request('PUT', "/api/storyboard/generations/{$item['id']}", $ai, ['file' => 'sb-25-1--g1.png', 'file_hash' => $hash, 'machine' => 'laptop']);
        self::assertSame([200, 'sb-25-1--g1.png', $hash, 'laptop'], [$saved->getStatusCode(), self::json($saved)['item']['file'], self::json($saved)['item']['file_hash'], self::json($saved)['item']['machine']]);
        $this->clock->advance(16 * 60);
        self::assertSame(404, $this->request('GET', $path)->getStatusCode(), 'the signed link ran out');
    }

    public function testTheGateCountsSpentAndReserved(): void
    {
        [$creator] = $this->auth([Roles::CREATOR]);
        $this->budget(1);
        // gen4.5 · 10 s = 120 credits = 1.20 USD ≈ 1.07 € > 1 €
        $r = $this->request('POST', '/api/storyboard/generations', $creator, self::runway('gen4.5', 10));
        self::assertSame([402, 'budget_exhausted', 1, 1.07, 0, 0], [$r->getStatusCode(), self::json($r)['error'], self::json($r)['budget'], self::json($r)['estimate'], self::json($r)['spent'], self::json($r)['reserved']]);
        self::assertSame([], $this->fake->calls('runway.test'));

        $this->budget(2);
        $a = $this->request('POST', '/api/storyboard/generations', $creator, self::runway());
        self::assertSame([201, 0.25, 'rw-1'], [$a->getStatusCode(), self::json($a)['item']['estimate'], self::json($a)['item']['job']], (string) $a->getBody());
        $sent = $this->fake->lastBody('/v1/image_to_video');
        self::assertSame(['gen4_turbo', 'https://cdn.scutoons.test/catpope/sb-25-2.png', '720:1280', 5, 'slow push in'], [$sent['model'], $sent['promptImage'], $sent['ratio'], $sent['duration'], $sent['promptText']]);
        self::assertSame(201, $this->request('POST', '/api/storyboard/generations', $creator, self::runway('gen4.5', 10))->getStatusCode(), '0.22 + 1.07 ≤ 2');
        $r = $this->request('POST', '/api/storyboard/generations', $creator, self::runway('gen4.5', 10));
        self::assertSame([402, 'budget_exhausted', 1.29], [$r->getStatusCode(), self::json($r)['error'], self::json($r)['reserved']], 'the running jobs are reserved');

        // a finished job books its actual cost (Runway reports credits) in place of the reservation
        $this->fake->runway['rw-1'] = ['id' => 'rw-1', 'status' => 'SUCCEEDED', 'output' => ['https://files.test/rw-1.mp4'], 'cost' => ['credits' => 20]];
        $item = self::json($this->request('GET', '/api/storyboard/generations/' . self::json($a)['item']['id'], $creator))['item'];
        self::assertSame(['succeeded', 0.2], [$item['status'], $item['cost']]);
        // raising the amount opens the gate at once
        $this->budget(3);
        self::assertSame(201, $this->request('POST', '/api/storyboard/generations', $creator, self::runway('gen4.5', 10))->getStatusCode());
    }

    public function testFailuresAndCancel(): void
    {
        [$creator] = $this->auth([Roles::CREATOR]);
        $this->budget(5);
        // cancel: Runway can (queued → nothing booked), Luma can't
        $rw = self::json($this->request('POST', '/api/storyboard/generations', $creator, self::runway()))['item'];
        $c = $this->request('POST', "/api/storyboard/generations/{$rw['id']}/cancel", $creator);
        self::assertSame([200, 'cancelled', 0], [$c->getStatusCode(), self::json($c)['item']['status'], self::json($c)['item']['cost']], (string) $c->getBody());
        self::assertContains('DELETE /v1/tasks/rw-1', $this->fake->calls('runway.test'));
        self::assertSame([409, 'not_open'], [$this->request('POST', "/api/storyboard/generations/{$rw['id']}/cancel", $creator)->getStatusCode(), 'not_open']);
        $luma = self::json($this->request('POST', '/api/storyboard/generations', $creator, self::LUMA))['item'];
        $c = $this->request('POST', "/api/storyboard/generations/{$luma['id']}/cancel", $creator);
        self::assertSame([409, 'not_cancellable'], [$c->getStatusCode(), self::json($c)['error']]);

        // a refunded failure books 0
        $this->fake->luma[$luma['job']] = ['id' => $luma['job'], 'state' => 'failed', 'failure_code' => 'content_moderated', 'failure_reason' => 'flagged'];
        $item = self::json($this->request('GET', "/api/storyboard/generations/{$luma['id']}", $creator))['item'];
        self::assertSame(['failed', 0, 'Luma: content_moderated flagged'], [$item['status'], $item['cost'], $item['error']]);
        self::assertSame(409, $this->request('GET', "/api/storyboard/generations/{$luma['id']}/result", $creator)->getStatusCode());

        // a refused start: 502, the row failed, nothing booked
        $this->fake->startStatus = 400;
        $r = $this->request('POST', '/api/storyboard/generations', $creator, self::LUMA);
        self::assertSame([502, 'provider_failed', 'failed', 0], [$r->getStatusCode(), self::json($r)['error'], self::json($r)['item']['status'], self::json($r)['item']['cost']]);
        self::assertStringContainsString('prompt rejected', self::json($r)['message']);
        // no answer: it may have been billed – the estimate is booked
        $this->fake->startStatus = 0;
        $r = $this->request('POST', '/api/storyboard/generations', $creator, self::LUMA);
        self::assertSame([502, 'failed', 0.0404], [$r->getStatusCode(), self::json($r)['item']['status'], self::json($r)['item']['cost']]);
        self::assertStringContainsString('may have been billed', self::json($r)['item']['error']);

        // an expired result link: 410
        $this->fake->startStatus = 200;
        $ok = self::json($this->request('POST', '/api/storyboard/generations', $creator, self::LUMA))['item'];
        $this->fake->luma[$ok['job']] = ['id' => $ok['job'], 'state' => 'completed', 'output' => [['type' => 'image', 'url' => 'https://files.test/expired.png']]];
        $this->request('GET', "/api/storyboard/generations/{$ok['id']}", $creator);
        self::assertSame(410, $this->request('GET', "/api/storyboard/generations/{$ok['id']}/result", $creator)->getStatusCode());
    }

    public function testJobChecks(): void
    {
        [$creator] = $this->auth([Roles::CREATOR]);
        $this->budget(5);
        $check = function (array $body, int $status, string $error, ?string $field = null) use ($creator): void {
            $r = $this->request('POST', '/api/storyboard/generations', $creator, $body);
            self::assertSame([$status, $error], [$r->getStatusCode(), self::json($r)['error']], (string) $r->getBody());
            if ($field !== null) self::assertSame($field, self::json($r)['field']);
        };
        $check(['model' => 'uni-9'] + self::LUMA, 422, 'invalid_job', 'model');
        $check(['prompt' => ''] + self::LUMA, 422, 'invalid_job', 'prompt');
        $check(['params' => ['style' => 'manga', 'ratio' => '16:9']] + self::LUMA, 422, 'invalid_job', 'params.style');
        $check(['references' => [['role' => 'still', 'work' => 'missing.png']]] + self::LUMA, 422, 'invalid_generation', 'references');
        $check(['references' => [['role' => 'still', 'url' => 'http://insecure.test/a.png']]] + self::LUMA, 422, 'invalid_generation', 'references');
        $check(['generator' => 'gemini-image'] + self::LUMA, 422, 'generator_unavailable');
        $check(self::runway('gen4_turbo', 5, ['references' => []]), 422, 'invalid_job', 'references');
        $check(self::runway('aleph2'), 422, 'invalid_job', 'references');
        $check(['seed' => 1] + self::LUMA, 422, 'invalid_generation', 'seed');
        // a record (with a file) under a provider's name stays a record; a job field there is refused
        $r = $this->request('POST', '/api/storyboard/generations', $creator, ['toon' => 'catpope', 'chapter' => 3, 'shot' => 'sb-1', 'kind' => 'video', 'generator' => 'runway', 'file' => 'sb-1--g1.mp4', 'machine' => 'pc', 'cost' => 0.5, 'currency' => 'USD']);
        self::assertSame([201, 'succeeded', null], [$r->getStatusCode(), self::json($r)['item']['status'], self::json($r)['item']['job']]);
        self::assertSame([], $this->fake->calls('luma.test'));
    }

    public function testAKeylessProviderAndTheAdminBudget(): void
    {
        [$admin] = $this->auth([Roles::ADMIN]);
        $r = $this->request('PUT', '/api/admin/settings', $admin, ['generation_budget' => 400]);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        self::assertSame([400, 400, 0], [self::json($r)['budget']['budget'], self::json($r)['budget']['left'], self::json($r)['budget']['spent']]);
        $items = array_column(self::json($this->request('GET', '/api/admin/settings', $admin))['items'], null, 'key');
        self::assertSame([400, 0], [$items['generation_budget']['value'], $items['generation_budget']['default']]);
        $r = $this->request('PUT', '/api/admin/settings', $admin, ['generation_budget' => -1]);
        self::assertStringContainsString('0 = no budget', self::json($r)['message']);
    }

    public function testTheEuroRateAndTheHelpers(): void
    {
        $this->fake->ecbXml = str_replace('1.25', '1.25', $this->fake->ecbXml);
        $rates = Kernel::container(Settings::fixed(self::root(), ['ECB_RATES_URL' => 'https://ecb.test/daily.xml'] + $this->settings()));
        $rates->set(Clock::class, $this->clock);
        $euro = $rates->get(EuroRates::class);
        $euro->useHttp($this->fake);
        self::assertSame(['ecb', 1.25, 0.8, '2026-10-20'], [$euro->describe()['source'], $euro->describe()['usd_per_eur'], $euro->describe()['eur_per_usd'], $euro->describe()['date']]);
        self::assertEqualsWithDelta(8.0, $euro->toEuro(10, 'USD'), 1e-9);
        $again = $rates->make(EuroRates::class);
        $again->useHttp($this->fake);
        $again->describe();
        self::assertCount(1, $this->fake->calls('ecb.test'), 'fetched at most once a day (cached)');
        self::assertNull(EuroRates::parse('<html>no rates</html>'));

        self::assertSame('720:1280', HttpProvider::nearestRatio('9:16', ['1280:720', '720:1280', '960:960']));
        self::assertSame('1:3', HttpProvider::nearestRatio('1:4', ['3:1', '1:1', '1:2', '1:3']));
        // an MP4 header: ftyp + moov → mvhd (version 0, timescale 1000, duration 4500)
        $mvhd = pack('N', 108) . 'mvhd' . "\0\0\0\0" . pack('NNNN', 0, 0, 1000, 4500) . str_repeat("\0", 80);
        $file = $this->dir . '/clip.mp4';
        file_put_contents($file, pack('N', 16) . 'ftypisom' . "\0\0\0\0" . pack('N', 8 + strlen($mvhd)) . 'moov' . $mvhd);
        self::assertSame(4.5, Mp4::seconds($file));
    }
}
