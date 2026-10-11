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
        self::assertSame([], $check('season:catpope:1', ['title' => ['de' => 'S'], 'sequence' => [['number' => 1, 'status' => ['de' => 'published', 'en' => 'coming-soon']]]]));
        self::assertNotSame([], $check('season:catpope:1', ['title' => ['de' => 'S'], 'sequence' => [['number' => 1, 'status' => 'published']]]));
        foreach (ContentSchemas::NEUTRAL_BY_KIND as $kind => $name) {
            self::assertContains($kind, EntityKey::NEUTRAL_DOC_KINDS);
            self::assertFileExists(self::root() . "/schema/content/$name.schema.json");
        }
    }

    /** expressions in texts (agents/EXPRESSIONS.md §4): syntax errors and the old `{x:clock}` block, doubts don't */
    public function testExpressionsInNeutralTexts(): void
    {
        $schemas = $this->get(ContentSchemas::class);
        $check = fn(string $key, mixed $content) => $schemas->problemsOf(EntityKey::parse($key), json_decode((string) json_encode($content)));
        $chapter = fn(array $dialog, array $message, array $label) => ['title' => ['de' => 'Eins'], 'author' => 'M', 'number' => 1, 'release' => ['de' => '2025-10-04'], 'description' => ['de' => ''],
            'thumbnail' => '', 'groups' => [['slices' => [['image' => 'a.webp', 'width' => 1, 'height' => 1, 'dialog' => [['rect' => ['x' => 0, 'y' => 0, 'width' => 1], 'text' => $dialog]],
                'items' => [['id' => 'i1', 'item' => 'coin', 'rect' => ['x' => 0, 'y' => 0, 'width' => 1, 'height' => 1], 'events' => [['type' => 'showMessage', 'params' => ['text' => $message]]]]]]],
                'events' => [['type' => 'addChoice', 'params' => ['label' => $label]]]]]];
        $fine = $chapter(['de' => ['{map.total - map.unlocked} Orte', '{map.totl}'], 'en' => '{coins == 1: coin|coins}'], ['de' => '{Hallo.|Du schon wieder?|Geh weg!} {clock(t)}'], ['de' => '{unknown_fn(1)} {coinz}']);
        self::assertSame([], $check('chapter:catpope:1', $fine), 'unknown names / functions are warnings in the creator, not a reason to refuse');
        $broken = $chapter(['de' => ['ok', 'Noch {map.total - } Orte']], ['de' => 'Es ist {time:clock} Uhr'], ['de' => 'Ja {coins', 'en' => 'fine']);
        self::assertSame([
            '/groups/0/slices/0/dialog/0/text/de/1: expected a value – at 19 (expr:expected-value)',
            '/groups/0/slices/0/items/0/events/0/params/text/de: `{time:clock}` is the old clock form – write `{clock(time)}` – at 8–19 (expr:old-clock-syntax)',
            '/groups/0/events/0/params/label/de: missing `}` – at 4–9 (expr:missing-close)',
        ], $check('chapter:catpope:1', $broken));
        self::assertNotSame([], $check('scene:catpope:1:kiosk', ['events' => [['type' => 'showMessage', 'params' => ['text' => ['de' => '{a +}']]]]]));
        self::assertSame([], $check('scene:catpope:1:kiosk', ['events' => [['type' => 'showMessage', 'params' => ['text' => ['de' => '{a + 1}']]]]]));
    }

    /** step 3: an event's `if` as an expression and an askText question – a syntax error blocks, a doubt doesn't */
    public function testExpressionConditionsAndAskText(): void
    {
        $schemas = $this->get(ContentSchemas::class);
        $check = fn(mixed $content) => $schemas->problemsOf(EntityKey::parse('scene:catpope:1:kiosk'), json_decode((string) json_encode($content)));
        $scene = fn(string $condition, string $question) => ['events' => [
            ['id' => 'e1', 'type' => 'showMessage', 'if' => ['expr' => $condition], 'params' => ['text' => ['de' => 'Hallo']]],
            ['id' => 'e2', 'type' => 'askText', 'params' => ['text' => ['de' => $question], 'scope' => 'reader']],
        ]];
        self::assertSame([], $check($scene('count("e1") < 3 and not has("cucumber")', 'Wie heißt du, {reader.name}?')));
        self::assertSame([], $check($scene('coinz > 2', 'Name?')), 'an unknown name is a doubt (a warning in the creator)');
        self::assertSame([
            '/events/0/if/expr: expected a value – at 10 (expr:expected-value)',
            '/events/1/params/text/de: missing `}` – at 5–11 (expr:missing-close)',
        ], $check($scene('coins > (', 'Und {reader')));
    }

    public function testABrokenExpressionRefusesTheSave(): void
    {
        $user = $this->user('creator@example.com');
        $this->get(Roles::class)->grant((string) $user['id'], Roles::CREATOR);
        $auth = ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])];
        $scene = fn(string $text) => (string) json_encode(['content' => ['events' => [['type' => 'showMessage', 'params' => ['text' => ['de' => $text]]]]]]);

        $bad = $this->request('PUT', '/api/entities/scene:smoke:1:kiosk', $auth, $scene('Hallo {name'));
        self::assertSame(422, $bad->getStatusCode(), (string) $bad->getBody());
        $body = json_decode((string) $bad->getBody(), true);
        self::assertStringContainsString("has a text that can't be shown: /events/0/params/text/de: missing `}`", (string) ($body['message'] ?? $body['error'] ?? ''));
        self::assertSame(['/events/0/params/text/de: missing `}` – at 7–11 (expr:missing-close)'], $body['problems'] ?? null);

        $good = $this->request('PUT', '/api/entities/scene:smoke:1:kiosk', $auth, $scene('Hallo {coinz}'));
        self::assertSame(200, $good->getStatusCode(), (string) $good->getBody());
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
    /**
     * Many documents at once (the world map's places / routes, agents/MAP-DOCUMENTS.md): POST /api/entities/batch stores
     * all or none (a bad document or a stale base names its key, nothing is stored), GET /api/entities?kinds= reads them
     * with their documents in creation order, POST /api/entities/publish puts several live at once
     */
    public function testBatchSaveReadAndPublish(): void
    {
        $user = $this->user('creator@example.com');
        $this->get(Roles::class)->grant((string) $user['id'], Roles::CREATOR);
        $auth = ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])];
        $place = fn(string $de, int $x) => ['name' => ['de' => $de], 'x' => $x, 'y' => 1, 'toons' => ['t']];
        $map = ['width' => 10, 'height' => 10, 'layers' => []];
        $post = fn(string $uri, array $body) => $this->request('POST', $uri, $auth, (string) json_encode($body));
        $read = fn(string $content = 'draft') => json_decode((string) $this->request('GET', "/api/entities?kinds=map,place,route&content=$content", $auth)->getBody(), true)['entities'];

        // a bad document: nothing is stored, the answer names it
        $bad = $post('/api/entities/batch', ['saves' => [['key' => 'map', 'content' => $map], ['key' => 'place:a', 'content' => ['x' => 'left']]]]);
        self::assertSame(422, $bad->getStatusCode());
        self::assertSame('place:a', json_decode((string) $bad->getBody(), true)['key']);
        self::assertSame([], $read());

        $ok = $post('/api/entities/batch', ['message' => 'two places', 'saves' => [
            ['key' => 'map', 'content' => $map, 'base' => null], ['key' => 'place:b', 'content' => $place('B', 2)], ['key' => 'place:a', 'content' => $place('A', 1)],
        ]]);
        self::assertSame(200, $ok->getStatusCode(), (string) $ok->getBody());
        $saved = json_decode((string) $ok->getBody(), true)['saved'];
        self::assertSame(['map', 'place:b', 'place:a'], array_column($saved, 'key'));
        $entities = $read();
        self::assertSame(['map', 'place:b', 'place:a'], array_column($entities, 'key'), 'creation order');
        self::assertSame('B', $entities[1]['content']['name']['de']);
        self::assertSame([null, null, null], array_column($read('published'), 'content'));

        // a stale base: 409 names the key, the other save of the batch is not stored either
        $stale = $post('/api/entities/batch', ['saves' => [['key' => 'place:a', 'content' => $place('A', 5), 'base' => $saved[2]['version']['id']], ['key' => 'place:b', 'content' => $place('B', 9), 'base' => 1]]]);
        self::assertSame(409, $stale->getStatusCode());
        self::assertSame('place:b', json_decode((string) $stale->getBody(), true)['key']);
        self::assertSame(1, $read()[2]['content']['x'], 'rolled back');

        $pub = $post('/api/entities/publish', ['versions' => array_map(fn($s) => ['key' => $s['key'], 'version' => $s['version']['id']], $saved)]);
        self::assertSame(200, $pub->getStatusCode(), (string) $pub->getBody());
        self::assertSame(['map', 'place:b', 'place:a'], array_column($read('published'), 'key'));
        self::assertSame('A', $read('published')[2]['content']['name']['de']);

        // a never-published place is deleted outright (owner D3), a published one is not (a tombstone instead)
        $post('/api/entities/batch', ['saves' => [['key' => 'place:x', 'content' => $place('X', 7)]]]);
        self::assertContains('place:x', array_column($read(), 'key'));
        $gone = $post('/api/entities/batch', ['saves' => [['key' => 'place:y', 'content' => $place('Y', 8)]], 'deletes' => ['place:x']]);
        self::assertSame(200, $gone->getStatusCode(), (string) $gone->getBody());
        self::assertSame(['place:x'], json_decode((string) $gone->getBody(), true)['deleted']);
        self::assertSame(['map', 'place:b', 'place:a', 'place:y'], array_column($read(), 'key'));
        $refused = $post('/api/entities/batch', ['saves' => [['key' => 'place:z', 'content' => $place('Z', 1)]], 'deletes' => ['place:a']]);
        self::assertSame(409, $refused->getStatusCode());
        self::assertSame('published', json_decode((string) $refused->getBody(), true)['error']);
        self::assertNotContains('place:z', array_column($read(), 'key'), 'nothing of a refused batch is stored');
        self::assertSame(400, $post('/api/entities/batch', ['deletes' => ['map']])->getStatusCode(), 'only places / routes are ever deleted');

        // the whole map in one document (an older client): refused, not silently ignored
        $old = $this->request('PUT', '/api/entities/map', $auth, (string) json_encode(['content' => $map + ['locations' => [['id' => 'a'] + $place('A', 1)]]]));
        self::assertSame(422, $old->getStatusCode());
    }
}
