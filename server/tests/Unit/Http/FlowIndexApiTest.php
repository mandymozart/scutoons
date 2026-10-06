<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/** GET /api/creator/flow-index (role creator): every toon's ways out of its chapters, seasons, dead things */
final class FlowIndexApiTest extends TestCase
{
    private function request(string $uri, string $token = ''): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest('GET', "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        if ($token !== '') $request = $request->withHeader('Authorization', "Bearer $token");
        $container = Kernel::container(Settings::fixed(self::root(), $this->settings()));
        $container->set(Clock::class, $this->clock);
        return Kernel::app($container)->handle($request);
    }

    private function token(string $role): string
    {
        $user = $this->user($role . '@example.com');
        if ($role !== 'member') $this->get(Roles::class)->grant((string) $user['id'], $role);
        return $this->get(Sessions::class)->create((string) $user['id']);
    }

    /** @param array<mixed> $doc */
    private function store(string $key, array $doc, bool $publish = true): void
    {
        $store = $this->get(EntityStore::class);
        $json = (string) json_encode($doc, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        if ($publish) $store->insertPublished(EntityKey::parse($key), $json, null);
        else $store->insert(EntityKey::parse($key), $json, null, 'draft', null);
    }

    public function testTheIndex(): void
    {
        $jump = fn(string $type, string $value, ?string $label = null) => ['type' => $type, 'value' => $value] + ($label !== null ? ['label' => $label] : []);
        $this->store('toon:catpope', ['id' => 'catpope', 'status' => 'published', 'title' => ['de' => 'Katzenpapst', 'en' => 'Cat Pope']]);
        $this->store('season:catpope:1', ['number' => 1, 'sequence' => [['type' => 'chapter', 'number' => 2, 'status' => ['de' => 'draft']], ['type' => 'chapter', 'number' => 3, 'status' => ['de' => 'draft']]]]);
        $this->store('chapter:catpope:2', ['number' => 2, 'groups' => [
            ['label' => 'start', 'continues' => true, 'slices' => [['events' => [['type' => 'jumpTo', 'params' => ['target' => $jump('product', 'vinyl')], 'trigger' => ['type' => 'click']]]]],
                'events' => [
                    ['type' => 'setVariable', 'params' => ['variable' => 'searching', 'value' => 'yes']],
                    ['type' => 'showMessage', 'params' => ['text' => ['de' => 'Zum {visits}. Mal']], 'if' => ['variable' => 'rocket', 'op' => 'set']],
                    // step 3: askText sets its variable (scope toon; scope reader: the reader's name), `if.expr` and `{…}` read
                    ['type' => 'askText', 'params' => ['text' => ['de' => 'Wie heißt du?'], 'variable' => 'hero']],
                    ['type' => 'askText', 'params' => ['text' => ['de' => 'Und du?'], 'variable' => 'ignored', 'scope' => 'reader']],
                    ['type' => 'showMessage', 'params' => ['text' => ['de' => '{gold > 2: reich|arm}, {reader.name}']], 'if' => ['expr' => 'coins >= 3 && hero != ""']],
                    ['type' => 'label', 'params' => ['name' => 'lonely']],
                    ['type' => 'addChoice', 'params' => ['label' => ['de' => 'Zu Top Toy'], 'onSelect' => [['add' => ['variable' => 'visits', 'value' => 1]], ['jumpTo' => ['target' => $jump('toon', 'toptoy')]]]]],
                    ['type' => 'addChoice', 'params' => ['label' => ['de' => 'Wald'], 'onSelect' => ['jumpTo' => ['target' => $jump('chapter', '3', 'forest')]]]],
                    ['type' => 'showChoices'],
                ]],
            ['label' => 'middle', 'slices' => [[]]],
            ['ref' => ['src' => 'chapter2/scene-9.json']],
        ]]);
        // a ref group: its scene's end; the newest version counts (a draft)
        $this->store('scene:catpope:2:scene-9', ['label' => 'scene-9', 'slices' => [[]], 'end' => ['target' => $jump('chapter', '3')]], false);
        $this->store('scene:catpope:2:scene-9', ['label' => 'scene-9', 'slices' => [[]], 'end' => ['target' => $jump('chapter', '3', 'forest'), 'auto' => true]], false);
        // a legacy per-locale chapter (no neutral document): read too
        $this->store('chapter:toptoy:1:de', ['number' => 1, 'groups' => [['label' => 'city', 'slices' => [[]], 'end' => ['target' => $jump('toon', 'catpope')]]]]);

        self::assertSame(401, $this->request('/api/creator/flow-index')->getStatusCode());
        self::assertSame(403, $this->request('/api/creator/flow-index', $this->token('member'))->getStatusCode());
        $token = $this->token('creator');
        $response = $this->request('/api/creator/flow-index', $token);
        self::assertSame(200, $response->getStatusCode());
        $index = (array) json_decode((string) $response->getBody(), true);

        self::assertSame(['catpope', 'toptoy'], array_column($index['toons'], 'id'));
        self::assertSame(['de' => 'Katzenpapst', 'en' => 'Cat Pope'], $index['toons'][0]['title']);
        self::assertSame([['number' => 1, 'chapters' => [['number' => 2, 'status' => ['de' => 'draft']], ['number' => 3, 'status' => ['de' => 'draft']]]]], $index['toons'][0]['seasons']);
        $edges = array_map(fn(array $e) => sprintf('%s:%d:%s %s %s:%s%s%s', $e['source']['toon'], $e['source']['chapter'], $e['source']['group'] ?? '-', $e['kind'],
            $e['target']['type'], $e['target']['value'], isset($e['target']['label']) ? '#' . $e['target']['label'] : '', isset($e['label']) ? ' [' . $e['label'] . ']' : ''), $index['edges']);
        self::assertSame([
            'catpope:2:start choice toon:toptoy [Zu Top Toy]',
            'catpope:2:start choice chapter:3#forest [Wald]',
            'catpope:2:start jump product:vinyl',
            'catpope:2:scene-9 end chapter:3#forest',
            'toptoy:1:city end toon:catpope',
        ], $edges);
        $dead = $index['dead']['catpope'];
        self::assertSame(['searching'], array_column($dead['unread'], 'name'));
        self::assertSame(['coins', 'gold', 'rocket'], array_column($dead['unset'], 'name'));
        self::assertSame([['name' => 'lonely', 'at' => [['chapter' => 2, 'group' => 0, 'label' => 'start']]]], $dead['points']);
        // `middle` follows a linked group and nothing jumps there; `scene-9` gets the follow-up bar
        self::assertSame(['middle'], array_column($dead['labels'], 'name'));

        // the same versions: the cached answer; a new version: a new answer
        self::assertSame($index, (array) json_decode((string) $this->request('/api/creator/flow-index', $token)->getBody(), true));
        $this->store('chapter:toptoy:1:de', ['number' => 1, 'groups' => [['label' => 'city', 'slices' => [[]]]]]);
        $again = (array) json_decode((string) $this->request('/api/creator/flow-index', $token)->getBody(), true);
        self::assertCount(4, $again['edges']);
    }

    /**
     * A chapter target with `toon` (owner 2026-10-06, "at label"): a chapter + label of another toon. The edge keeps the
     * toon; the label is an entry point of THAT toon (not a dead label there), and no entry of the toon it leaves from.
     */
    public function testJumpsIntoAnotherToonsLabel(): void
    {
        $into = fn(string $toon, string $chapter, string $label) => ['type' => 'chapter', 'value' => $chapter, 'toon' => $toon, 'label' => $label];
        // catpope: `middle` follows a linked group – only toptoy's jump names a label "middle" (toptoy's, not this one)
        $this->store('chapter:catpope:3', ['number' => 3, 'groups' => [
            ['label' => 'forest', 'continues' => true, 'slices' => [[]], 'events' => [
                ['type' => 'addChoice', 'params' => ['label' => ['de' => 'Hyper City'], 'onSelect' => ['jumpTo' => ['target' => $into('toptoy', '1', 'hyper')]]]],
                ['type' => 'addChoice', 'params' => ['label' => ['de' => 'Mitte'], 'onSelect' => ['jumpTo' => ['target' => $into('toptoy', '1', 'middle')]]]],
                ['type' => 'showChoices'],
            ]],
            ['label' => 'middle', 'slices' => [[]]],
            ['label' => 'biker', 'slices' => [[]]],
        ]]);
        // toptoy: `hyper` and `middle` follow linked groups – reached from catpope only; `lost` from nowhere
        $this->store('chapter:toptoy:1', ['number' => 1, 'groups' => [
            ['label' => 'city', 'continues' => true, 'slices' => [[]]],
            ['label' => 'hyper', 'continues' => true, 'slices' => [[]]],
            ['label' => 'middle', 'continues' => true, 'slices' => [[]]],
            ['label' => 'lost', 'slices' => [[]], 'end' => ['target' => $into('catpope', '3', 'biker')]],
        ]]);
        $response = $this->request('/api/creator/flow-index', $this->token('creator'));
        self::assertSame(200, $response->getStatusCode());
        $index = (array) json_decode((string) $response->getBody(), true);
        $targets = array_map(fn(array $e) => $e['source']['toon'] . ' → ' . json_encode($e['target']), $index['edges']);
        self::assertSame([
            'catpope → {"type":"chapter","value":"1","toon":"toptoy","label":"hyper"}',
            'catpope → {"type":"chapter","value":"1","toon":"toptoy","label":"middle"}',
            'toptoy → {"type":"chapter","value":"3","toon":"catpope","label":"biker"}',
        ], $targets);
        // catpope's `middle` is dead (the jump names toptoy's); `biker` follows the follow-up bar anyway
        self::assertSame(['middle'], array_column($index['dead']['catpope']['labels'], 'name'));
        // toptoy's `hyper` / `middle` are reached from catpope; `lost` from nowhere
        self::assertSame(['lost'], array_column($index['dead']['toptoy']['labels'], 'name'));
    }
}
