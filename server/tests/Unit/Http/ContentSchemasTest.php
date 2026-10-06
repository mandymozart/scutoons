<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Content\ContentSchemas;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/** The type guard of content saves (ContentSchemas): PUT /api/entities/<key> refuses documents that don't fit their kind */
final class ContentSchemasTest extends TestCase
{
    /** @param array<string, string> $headers */
    private function request(string $method, string $uri, array $headers = [], ?string $body = null): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        if ($body !== null) {
            $request->getBody()->write($body);
            $request->getBody()->rewind();
            $request = $request->withHeader('Content-Type', 'application/json');
        }
        $container = Kernel::container(Settings::fixed(self::root(), $this->settings()));
        $container->set(Clock::class, $this->clock);
        return Kernel::app($container)->handle($request);
    }

    private const EVENT = ['title' => 'Cat Pope', 'date' => '2026-11-14T20:00:00', 'location' => 'Arena', 'city' => 'Vienna', 'category' => 'irl', 'toon' => ['catpope']];

    /**
     * @param mixed $content decoded as the API decodes it (objects as stdClass)
     * @return list<string>
     */
    private function problems(string $kind, mixed $content): array
    {
        return $this->get(ContentSchemas::class)->problems($kind, json_decode((string) json_encode($content)));
    }

    public function testEveryKindHasItsSchemaOrIsExplicitlyUnchecked(): void
    {
        foreach (ContentSchemas::BY_KIND as $kind => $name) {
            if ($name !== null) self::assertFileExists(self::root() . "/schema/content/$name.schema.json", $kind);
        }
        self::assertSame(array_keys(\Scutoons\Domain\Content\EntityKey::KINDS), array_keys(ContentSchemas::BY_KIND), 'every entity kind is mapped');
    }

    public function testEvents(): void
    {
        self::assertSame([], $this->problems('events', [self::EVENT, ['link' => 'https://x.test'] + self::EVENT]));
        self::assertSame([], $this->problems('events', []));
        $missing = self::EVENT;
        unset($missing['city']);
        self::assertStringContainsString('city', implode(' ', $this->problems('events', [self::EVENT, $missing])));
        self::assertStringContainsString('/1/category', implode(' ', $this->problems('events', [self::EVENT, ['category' => 'party'] + self::EVENT])));
        self::assertNotSame([], $this->problems('events', ['events' => [self::EVENT]]), 'an object is not the list');
        self::assertNotSame([], $this->problems('events', [['date' => 20261114] + self::EVENT]), 'date is a string');
    }

    public function testStoredShapesThatTheReadersAccept(): void
    {
        // narration (no speaker), a bare scene reference, a key visual as a plain path – as stored for catpope
        $group = ['events' => [['type' => 'showMessage', 'params' => ['text' => 'Ein sonniger Sonntag.', 'wait' => true]]]];
        self::assertSame([], $this->problems('scene', $group));
        self::assertNotSame([], $this->problems('scene', ['events' => [['type' => 'showMessage', 'params' => ['wait' => true]]]]), 'text is required');
        self::assertSame([], $this->problems('locations', ['anything' => 1]), 'the old map is not checked');
    }

    /** chapter:<toon>:<n> (no locale, agents/NEUTRAL.md) is checked against chapter-doc, chapter:<toon>:<n>:<l> against chapter */
    public function testNeutralDocumentsByKey(): void
    {
        $schemas = $this->get(ContentSchemas::class);
        $check = fn(string $key, mixed $content) => $schemas->problemsOf(EntityKey::parse($key), json_decode((string) json_encode($content)));
        $neutral = ['title' => ['de' => 'Eins', 'en' => 'One'], 'author' => 'M', 'number' => 1, 'release' => ['de' => '2025-10-04'], 'description' => ['de' => ''],
            'thumbnail' => '', 'groups' => [['slices' => [['image' => 'a.webp', 'width' => 1, 'height' => 1, 'dialog' => [['rect' => ['x' => 0, 'y' => 0, 'width' => 1], 'text' => ['de' => ['a', 'b'], 'en' => 'ab']]]]],
                'events' => [['type' => 'addChoice', 'params' => ['label' => ['de' => 'Ja']]]]]]];
        self::assertSame([], $check('chapter:catpope:1', $neutral));
        self::assertNotSame([], $check('chapter:catpope:1:de', $neutral), 'per locale: texts are strings');
        $perLocale = ['language' => 'de', 'title' => 'Eins'] + $neutral;
        self::assertNotSame([], $check('chapter:catpope:1', ['title' => 'Eins'] + $neutral), 'neutral: a title is a locale map');
        self::assertNotSame([], $check('chapter:catpope:1', $perLocale));
        self::assertSame([], $check('scene:catpope:1:kiosk', ['events' => [['type' => 'showMessage', 'params' => ['text' => ['de' => 'Na?'], 'mood' => 'sad']]]]));
        self::assertNotSame([], $check('scene:catpope:1:kiosk', ['events' => [['type' => 'showMessage', 'params' => ['text' => 'Na?']]]]));
        self::assertSame([], $check('season:catpope:1', ['title' => ['de' => 'S'], 'sequence' => [['type' => 'chapter', 'number' => 1, 'status' => ['de' => 'published', 'en' => 'coming-soon']]]]));
        self::assertNotSame([], $check('season:catpope:1', ['title' => ['de' => 'S'], 'sequence' => [['type' => 'chapter', 'number' => 1, 'status' => 'published']]]));
        foreach (ContentSchemas::NEUTRAL_BY_KIND as $kind => $name) {
            self::assertContains($kind, EntityKey::NEUTRAL_DOC_KINDS);
            self::assertFileExists(self::root() . "/schema/content/$name.schema.json");
        }
    }

    public function testNewToonSettingsWithoutAcceptedWarnings(): void
    {
        // as the creator's "Create season 1" writes them (newToon): no accepted_warnings until a warning is accepted
        $settings = ['id' => 'catpope', 'title' => 'Cat Pope', 'language' => 'de', 'release' => '2026-10-06', 'status' => 'draft', 'description' => ''];
        self::assertSame([], $this->problems('toon-settings', $settings));
        self::assertSame([], $this->problems('toon-settings', $settings + ['accepted_warnings' => ['title#1a2b3c4d']]));
        self::assertNotSame([], $this->problems('toon-settings', $settings + ['accepted_warnings' => 'title']), 'a list of strings');
    }

    public function testThePutIsRefused(): void
    {
        $user = $this->user('creator@example.com');
        $this->get(Roles::class)->grant((string) $user['id'], Roles::CREATOR);
        $auth = ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])];

        $bad = $this->request('PUT', '/api/entities/events', $auth, (string) json_encode(['content' => [['title' => 'x']]]));
        self::assertSame(422, $bad->getStatusCode());
        $body = json_decode((string) $bad->getBody(), true);
        self::assertSame('invalid_content', $body['code'] ?? $body['error'] ?? null);
        self::assertNotEmpty($body['problems'] ?? null);

        $good = $this->request('PUT', '/api/entities/events', $auth, (string) json_encode(['content' => [self::EVENT]]));
        self::assertSame(200, $good->getStatusCode(), (string) $good->getBody());
    }
}
