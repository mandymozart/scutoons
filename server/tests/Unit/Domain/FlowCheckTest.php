<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Content\FlowCheck;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * Where a reader could get stuck – refused on save (owner 2026-10-06, @scutoons/content stuck.ts + validate.ts the same):
 * a question whose choices can all be hidden, a divert to a missing point, a jump to a missing chapter / label / toon
 */
final class FlowCheckTest extends TestCase
{
    /** @param array<mixed> $doc */
    private function store(string $key, array $doc): void
    {
        $this->get(EntityStore::class)->insert(EntityKey::parse($key), (string) json_encode($doc, JSON_UNESCAPED_SLASHES), null, 'draft', null);
    }

    /**
     * @param array<mixed> $doc
     * @return list<string>
     */
    private function problems(string $key, array $doc): array
    {
        return $this->get(FlowCheck::class)->problems(EntityKey::parse($key), json_decode((string) json_encode($doc)));
    }

    /**
     * @param list<array<mixed>> $events
     * @param list<array<mixed>> $more
     * @return array<string, mixed>
     */
    private static function chapter(array $events, array $more = []): array
    {
        return ['number' => 2, 'groups' => [['label' => 'start', 'slices' => [[]], 'events' => $events], ...$more]];
    }

    public function testQuestionsWhoseChoicesCanAllBeHidden(): void
    {
        $choice = fn(string $label, ?array $if = null) => ['type' => 'addChoice', 'params' => ['label' => ['de' => $label]]] + ($if ? ['if' => $if] : []);
        $show = ['type' => 'showChoices', 'params' => new \stdClass()];
        $hidden = $this->problems('chapter:catpope:2', self::chapter([$choice('A', ['variable' => 'x', 'op' => 'set']), $choice('B', ['expr' => 'y > 2']), $show]));
        self::assertCount(1, $hidden);
        self::assertStringStartsWith('/groups/0/events/2: every choice of this question has a condition', $hidden[0]);
        self::assertSame([], $this->problems('chapter:catpope:2', self::chapter([$choice('A', ['variable' => 'x', 'op' => 'set']), $choice('Go on'), $show])), 'one unconditional choice');
        // the questions of a slice trigger and of a placed item's taps too; an unconditional divert ends a question's choices
        $slice = ['events' => [$choice('A', ['variable' => 'x', 'op' => 'set']), $show], 'items' => [['id' => 'i', 'item' => 'cucumber', 'action' => 'events', 'events' => [$choice('B', ['expr' => 'x']), $show]]]];
        $doc = ['number' => 2, 'groups' => [['label' => 'start', 'slices' => [$slice], 'events' => [$choice('C'), ['type' => 'divert', 'params' => ['to' => 'p']], ['type' => 'label', 'params' => ['name' => 'p']], $choice('D', ['expr' => 'x']), $show]]]];
        $found = $this->problems('chapter:catpope:2', $doc);
        self::assertSame(['/groups/0/events/4', '/groups/0/slices/0/events/1', '/groups/0/slices/0/items/0/events/1'], array_map(fn($p) => explode(':', $p)[0], $found));
    }

    public function testDivertsAndJumpsIntoNothing(): void
    {
        $this->store('toon:toptoy', ['id' => 'toptoy']);
        $this->store('chapter:toptoy:1', ['number' => 1, 'groups' => [['label' => 'hyper', 'slices' => [[]]], ['ref' => ['src' => 'chapter1/boss.json']]]]);
        $this->store('scene:toptoy:1:boss', ['label' => 'boss', 'slices' => [[]]]);
        $this->store('chapter:catpope:3', ['number' => 3, 'groups' => [['label' => 'forest', 'slices' => [[]]]]]);
        $jump = fn(array $target) => ['type' => 'jumpTo', 'params' => ['target' => $target]];
        $doc = self::chapter([
            ['type' => 'divert', 'params' => ['to' => 'nowhere']],
            $jump(['type' => 'chapter', 'value' => '2', 'label' => 'start']), // this chapter, its new content
            $jump(['type' => 'chapter', 'value' => '2', 'label' => 'later']), // a group of the saved content
            $jump(['type' => 'chapter', 'value' => '2', 'label' => 'gone']),
            $jump(['type' => 'chapter', 'value' => '3', 'label' => 'forest']),
            $jump(['type' => 'chapter', 'value' => '9']),
            $jump(['type' => 'chapter', 'value' => '1', 'toon' => 'toptoy', 'anyOf' => ['hyper', 'boss', 'nope']]),
            $jump(['type' => 'chapter', 'value' => '1', 'toon' => 'nobody', 'label' => 'x']),
            $jump(['type' => 'toon', 'value' => 'toptoy']),
            $jump(['type' => 'toon', 'value' => 'nobody']),
            ['type' => 'addChoice', 'params' => ['label' => ['de' => 'x'], 'onSelect' => [['divert' => ['to' => 'missing']], ['jumpTo' => ['target' => ['type' => 'chapter', 'value' => '3', 'label' => 'lake']]]]]],
            ['type' => 'showChoices'],
        ], [['label' => 'later', 'slices' => [[]], 'end' => ['target' => ['type' => 'chapter', 'value' => '3', 'label' => 'cave']]]]);
        $found = $this->problems('chapter:catpope:2', $doc);
        self::assertSame([
            '/groups/0/events/0/params/to: there is no point "nowhere" in this group\'s script (stuck:missing-point)',
            '/groups/0/events/3/params/target/label: chapter 2 has no group labelled "gone" (stuck:missing-label)',
            '/groups/0/events/5/params/target/value: the toon has no chapter 9 (stuck:missing-chapter)',
            '/groups/0/events/6/params/target/anyOf: toptoy chapter 1 has no group labelled "nope" (stuck:missing-label)',
            '/groups/0/events/7/params/target/toon: there is no toon "nobody" (stuck:missing-toon)',
            '/groups/0/events/9/params/target/value: there is no toon "nobody" (stuck:missing-toon)',
            '/groups/0/events/10/params/onSelect/0/divert/to: there is no point "missing" in this group\'s script (stuck:missing-point)',
            '/groups/0/events/10/params/onSelect/1/jumpTo/target/label: chapter 3 has no group labelled "lake" (stuck:missing-label)',
            '/groups/1/end/target/label: chapter 3 has no group labelled "cave" (stuck:missing-label)',
        ], $found);
        // a scene: its own label counts with its new content
        self::assertSame([], $this->problems('scene:toptoy:1:boss', ['label' => 'big-boss', 'slices' => [[]], 'events' => [$jump(['type' => 'chapter', 'value' => '1', 'label' => 'big-boss'])]]));
        self::assertSame([], $this->problems('chapter:catpope:2:de', $doc), 'legacy per-locale documents are not checked');
    }

    public function testTheSaveIsRefused(): void
    {
        $user = $this->user('creator@example.com');
        $this->get(Roles::class)->grant((string) $user['id'], 'creator');
        $token = $this->get(Sessions::class)->create((string) $user['id']);
        $put = function (array $content) use ($token): ResponseInterface {
            $request = (new ServerRequestFactory())->createServerRequest('PUT', 'http://localhost/api/entities/chapter:catpope:2', ['REMOTE_ADDR' => '127.0.0.1'])
                ->withHeader('Authorization', "Bearer $token")->withHeader('Content-Type', 'application/json');
            $request->getBody()->write((string) json_encode(['content' => $content]));
            $request->getBody()->rewind();
            $container = Kernel::container(Settings::fixed(self::root(), $this->settings()));
            $container->set(Clock::class, $this->clock);
            return Kernel::app($container)->handle($request);
        };
        $group = ['label' => 'start', 'renderer' => ['type' => 'vertical'], 'slices' => [['image' => 'a.webp', 'width' => 1, 'height' => 1]], 'events' => [
            ['type' => 'addChoice', 'params' => ['label' => ['de' => 'A']], 'if' => ['variable' => 'x', 'op' => 'set']],
            ['type' => 'showChoices', 'params' => new \stdClass()],
        ]];
        $neutral = ['number' => 2, 'title' => ['de' => 'Zwei'], 'author' => 'A', 'release' => ['de' => '2026-01-01'], 'description' => ['de' => ''], 'thumbnail' => '', 'groups' => [$group]];
        $refused = $put($neutral);
        self::assertSame(422, $refused->getStatusCode(), (string) $refused->getBody());
        $body = json_decode((string) $refused->getBody(), true);
        self::assertSame('invalid_content', $body['error'] ?? null, (string) $refused->getBody());
        self::assertStringContainsString('can get the reader stuck', (string) $refused->getBody());
        $group['events'][] = ['type' => 'addChoice', 'params' => ['label' => ['de' => 'Weiter']]];
        $group['events'][] = ['type' => 'showChoices', 'params' => new \stdClass()];
        $group['events'][1] = ['type' => 'addChoice', 'params' => ['label' => ['de' => 'B']]];
        $neutral['groups'] = [$group];
        $ok = $put($neutral);
        self::assertSame(200, $ok->getStatusCode(), (string) $ok->getBody());
    }
}
