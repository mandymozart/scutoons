<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Catalog\CatalogValidator;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Support\Time;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * XR experiences (#482, research/XR.md §4.4): the catalog kind `experiences` (documents `experience:<id>`) – the shallow
 * check of a write, drafts through /api/entities, publish writes `catalog_experiences`, the reader reads the published
 * one at GET /api/library/experiences/<id>.
 */
final class XrExperiencesTest extends TestCase
{
    /**
     * @param array<string, mixed> $fields
     * @return array<string, mixed> the smallest valid experience (`targets: {}` stays an object through the API)
     */
    private static function experience(array $fields = []): array
    {
        return $fields + [
            'id' => 'poster-catpope', 'type' => 'poster', 'title' => ['en' => 'Cat Pope', 'de' => 'Katzenpapst'], 'toons' => ['catpope'], 'assets' => 'xr/poster-catpope/e8f9fbfd81b2',
            'configuration' => ['version' => ['schema' => '5.1', 'app' => '4.1.0'], 'scenes' => [['id' => 'poster', 'targets' => new \stdClass()]], 'entries' => []],
            'plugins' => [],
        ];
    }

    /**
     * @param array<string, mixed> $data
     * @return list<string>
     */
    private function problems(array $data, string $id = 'poster-catpope'): array
    {
        $json = (string) json_encode($data);
        return $this->get(CatalogValidator::class)->problems('experiences', $id, json_decode($json, true), json_decode($json), false);
    }

    public function testTheTypesAreTheContentPackagesReadFromItsSchema(): void
    {
        self::assertSame(['book', 'record', 'poster', 'exhibition', 'concert-reading'], CatalogValidator::experienceTypes());
    }

    public function testAMinimalAndAFullExperienceAreAccepted(): void
    {
        self::assertSame([], $this->problems(self::experience()));
        $full = self::experience([
            'source' => ['osct' => ['experience' => 'demo-book', 'scene' => 'catpope', 'build' => '42', 'schema' => '5.1', 'app' => '4.1.0']],
            'plugins' => [['id' => 'toon', 'api' => '3.1', 'code' => 'export default {}'], ['id' => 'hover-2', 'api' => '3.0', 'code' => str_repeat('x', CatalogValidator::XR_PLUGIN_MAX_BYTES)]],
            'toons' => [],
        ]);
        self::assertSame([], $this->problems($full));
    }

    public function testEachBrokenFieldIsRefused(): void
    {
        $plugin = ['id' => 'toon', 'api' => '3.1', 'code' => 'export default {}'];
        $config = self::experience()['configuration'];
        $cases = [
            'id "Poster" is not a valid experiences id' => [self::experience(['id' => 'Poster']), 'Poster'],
            'data.id must be "poster-catpope"' => [self::experience(['id' => 'other'])],
            'an experience has no field kind' => [self::experience(['kind' => 'poster'])],
            'type must be one of book, record, poster, exhibition, concert-reading' => [self::experience(['type' => 'flyer'])],
            'title must be { <locale>: text }' => [self::experience(['title' => 'Cat Pope'])],
            'toons must be a list of toon ids' => [self::experience(['toons' => 'catpope'])],
            'source must be { osct: { experience, scene?, build, schema, app } } (texts)' => [self::experience(['source' => ['osct' => ['experience' => 'demo-book', 'build' => 42, 'schema' => '5.1', 'app' => '4.1.0']]])],
            'assets must be the CDN folder of its files (`xr/<id>/<build>`, no slash at either end)' => [self::experience(['assets' => '/xr/poster-catpope/'])],
            'configuration must be OSCT\'s game configuration (an object)' => [self::experience(['configuration' => 'game.config.json'])],
            'configuration.version.schema must be a schema version like "5.0"' => [self::experience(['configuration' => ['version' => ['schema' => '5']] + $config])],
            'configuration.scenes must be a list of at least one scene' => [self::experience(['configuration' => ['scenes' => []] + $config])],
            'configuration.entries must be a list' => [self::experience(['configuration' => ['entries' => ['a' => 1]] + $config])],
            'plugins must be a list' => [self::experience(['plugins' => ['toon' => $plugin]])],
            'plugin Toon: id must match ^[a-z][a-z0-9-]{0,39}$' => [self::experience(['plugins' => [['id' => 'Toon'] + $plugin]])],
            'plugin toon: the id is used twice' => [self::experience(['plugins' => [$plugin, $plugin]])],
            'plugin toon: api must be a plugin API version like "3.1"' => [self::experience(['plugins' => [['api' => '3'] + $plugin]])],
            'plugin toon: code must be the compiled module (a text)' => [self::experience(['plugins' => [['code' => ' '] + $plugin]])],
            'plugin toon: code is larger than 262144 bytes' => [self::experience(['plugins' => [['code' => str_repeat('x', CatalogValidator::XR_PLUGIN_MAX_BYTES + 1)] + $plugin]])],
        ];
        foreach ($cases as $problem => $case) {
            self::assertSame([$problem], $this->problems($case[0], $case[1] ?? 'poster-catpope'), $problem);
        }
        foreach ([['en' => ''], ['english' => 'Cat Pope'], [], ['Cat Pope']] as $title) {
            self::assertSame(['title must be { <locale>: text }'], $this->problems(self::experience(['title' => $title])), (string) json_encode($title));
        }
        self::assertSame(['toons must be a list of toon ids'], $this->problems(self::experience(['toons' => ['Cat Pope!']])));
        $assets = 'assets must be the CDN folder of its files (`xr/<id>/<build>`, no slash at either end)';
        foreach (['', 'xr//poster', 'xr/poster catpope', 'xr/../catpope', 'xr/poster/..', 'xr/./poster'] as $folder) {
            self::assertSame([$assets], $this->problems(self::experience(['assets' => $folder])), $folder);
        }
        $without = self::experience();
        unset($without['assets']);
        self::assertSame([$assets], $this->problems($without), 'assets is required');
    }

    public function testADraftPublishesIntoTheTableAndReadersReadThePublishedOne(): void
    {
        $writer = $this->role('writer@example.com', Roles::CREATOR);
        $doc = self::experience();
        self::assertSame(200, $this->request('PUT', '/api/entities/experience:poster-catpope', $writer, (string) json_encode(['content' => $doc]))->getStatusCode());
        $bad = $this->request('PUT', '/api/entities/experience:poster-catpope', $writer, (string) json_encode(['content' => self::experience(['type' => 'flyer'])]));
        self::assertSame([422, 'invalid_catalog'], [$bad->getStatusCode(), self::json($bad)['error'] ?? null]);
        self::assertNull($this->db->value("SELECT data FROM catalog_experiences WHERE id = 'poster-catpope'"), 'a draft stays out of the catalog');

        $none = $this->request('GET', '/api/library/experiences/poster-catpope');
        self::assertSame([404, 'not_found'], [$none->getStatusCode(), self::json($none)['error'] ?? null], 'not published yet');
        self::assertSame(401, $this->request('GET', '/api/library/experiences/poster-catpope?draft=1')->getStatusCode());
        $preview = $this->request('GET', '/api/library/experiences/poster-catpope?draft=1', $writer);
        self::assertSame([200, 'private, no-store'], [$preview->getStatusCode(), $preview->getHeaderLine('Cache-Control')]);
        self::assertSame('Cat Pope', self::json($preview)['experience']['title']['en']);

        $published = $this->request('POST', '/api/entities/experience:poster-catpope/publish', $writer, '{}');
        self::assertSame(200, $published->getStatusCode(), (string) $published->getBody());
        self::assertNotNull($this->db->value("SELECT data FROM catalog_experiences WHERE id = 'poster-catpope'"));

        $read = $this->request('GET', '/api/library/experiences/poster-catpope');
        self::assertSame([200, 'public, max-age=60'], [$read->getStatusCode(), $read->getHeaderLine('Cache-Control')]);
        $answer = self::json($read);
        self::assertSame(['experience', 'updatedAt'], array_keys($answer));
        self::assertEquals(json_decode((string) json_encode($doc), true), $answer['experience']);
        self::assertSame(Time::iso($this->db->value("SELECT updated_at FROM catalog_experiences WHERE id = 'poster-catpope'")), $answer['updatedAt']);
        self::assertStringContainsString('"targets":{}', (string) $read->getBody(), 'the configuration keeps its objects');
        self::assertSame(304, $this->request('GET', '/api/library/experiences/poster-catpope', [], null, ['If-None-Match' => $read->getHeaderLine('ETag')])->getStatusCode());

        self::assertSame(404, $this->request('GET', '/api/library/experiences/poster-nope')->getStatusCode());
        self::assertSame(404, $this->request('GET', '/api/library/experiences/Poster-Catpope')->getStatusCode(), 'not an experience id');
    }

    public function testTheKindIsInTheCreatorsAndTheAdminsCatalogRoutes(): void
    {
        $writer = $this->role('writer@example.com', Roles::CREATOR);
        self::assertSame(200, $this->request('PUT', '/api/entities/experience:poster-catpope', $writer, (string) json_encode(['content' => self::experience()]))->getStatusCode());
        $docs = $this->request('GET', '/api/catalog/docs/experiences', $writer);
        self::assertSame(200, $docs->getStatusCode(), (string) $docs->getBody());
        self::assertSame(['poster-catpope'], array_column(self::json($docs)['entries'], 'id'));

        // an admin reads the catalog's list; the entry itself is edited as a document (drafts, versions, review)
        $admin = $this->role('boss@example.com', Roles::ADMIN);
        $put = $this->request('PUT', '/api/admin/catalog/experiences/poster-book', $admin, (string) json_encode(['data' => self::experience(['id' => 'poster-book', 'type' => 'book'])]));
        self::assertSame([409, 'edited_in_creator'], [$put->getStatusCode(), self::json($put)['error'] ?? null], (string) $put->getBody());
        self::assertSame(200, $this->request('POST', '/api/entities/experience:poster-catpope/publish', $writer, '{}')->getStatusCode());
        self::assertSame(['poster-catpope'], array_column(self::json($this->request('GET', '/api/admin/catalog/experiences', $admin))['items'], 'id'));
    }

    /** @return array<string, mixed> */
    private function role(string $email, string $role): array
    {
        $user = $this->user($email);
        $this->get(Roles::class)->grant((string) $user['id'], $role);
        return $user;
    }

    /** @return array<string, mixed> */
    private static function json(ResponseInterface $response): array
    {
        return (array) json_decode((string) $response->getBody(), true);
    }

    /**
     * @param array<string, mixed> $user
     * @param array<string, string> $headers
     */
    private function request(string $method, string $uri, array $user = [], ?string $body = null, array $headers = []): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        if ($user !== []) $request = $request->withHeader('Authorization', 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id']));
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        if ($body !== null) {
            $request = $request->withHeader('Content-Type', 'application/json');
            $request->getBody()->write($body);
            $request->getBody()->rewind();
        }
        // a container per request, like the API: no catalog rows kept from an earlier one
        $container = Kernel::container(Settings::fixed(self::root(), $this->settings()));
        $container->set(Clock::class, $this->clock);
        return Kernel::app($container)->handle($request);
    }
}
