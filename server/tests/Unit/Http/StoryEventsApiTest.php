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

/** GET /api/creator/story-events (role creator): the story events every toon's chapters fire (#521) */
final class StoryEventsApiTest extends TestCase
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
    private function store(string $key, array $doc): void
    {
        $json = (string) json_encode($doc, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        $this->get(EntityStore::class)->insertPublished(EntityKey::parse($key), $json, null);
    }

    public function testTheStoryEventsPerChapter(): void
    {
        // Cat Pope's chapter 2: a joinScene in a slice's events (named by its persona's character), a showMessage without
        // an id, in the second group a leaveScene and a long message without a speaker
        $this->store('chapter:catpope:2', ['number' => 2, 'groups' => [
            ['label' => 'start', 'continues' => true, 'slices' => [['events' => [
                ['id' => 'join-cat', 'type' => 'joinScene', 'params' => ['characters' => [['character' => 'catpope', 'persona' => 'biker']]]],
            ]]], 'events' => [
                ['type' => 'showMessage', 'params' => ['character' => 'catpope', 'text' => ['de' => 'Miau, Freunde!']]],
            ]],
            ['label' => 'end', 'slices' => [[]], 'events' => [
                ['id' => 'bye', 'type' => 'leaveScene', 'params' => ['characters' => [['character' => 'catpope'], ['name' => 'plf']]]],
                ['id' => 'long', 'type' => 'showMessage', 'params' => ['text' => ['en' => str_repeat('a', 100)]]],
                ['type' => 'jumpTo', 'params' => ['target' => ['type' => 'chapter', 'value' => '3']]],
            ]],
        ]]);
        // toptoy's chapter 1: a setVariable (a story event type too) with an id; a group without events
        $this->store('chapter:toptoy:1', ['number' => 1, 'groups' => [
            ['label' => 'city', 'slices' => [[]], 'events' => [['id' => 'set1', 'type' => 'setVariable', 'params' => ['variable' => 'visits', 'value' => 1]]]],
        ]]);
        // a chapter of a toon without a story event: listed with no events
        $this->store('chapter:toptoy:2', ['number' => 2, 'groups' => [['slices' => [[]]]]]);

        self::assertSame(401, $this->request('/api/creator/story-events')->getStatusCode());
        self::assertSame(403, $this->request('/api/creator/story-events', $this->token('member'))->getStatusCode());
        $response = $this->request('/api/creator/story-events', $this->token('creator'));
        self::assertSame(200, $response->getStatusCode());
        $index = (array) json_decode((string) $response->getBody(), true);

        self::assertSame('success', $index['status']);
        self::assertSame(['catpope', 'toptoy'], array_column($index['toons'], 'id'));
        $catpope = $index['toons'][0]['chapters'];
        self::assertSame([2], array_column($catpope, 'number'));
        $events = $catpope[0]['events'];
        // document order: the group's slice events, then its own events; `group` is the label, `groupIndex` its position
        self::assertSame([
            ['id' => 'join-cat', 'type' => 'joinScene', 'characters' => ['catpope'], 'excerpt' => null, 'group' => 'start', 'groupIndex' => 0],
            ['id' => null, 'type' => 'showMessage', 'characters' => ['catpope'], 'excerpt' => 'Miau, Freunde!', 'group' => 'start', 'groupIndex' => 0],
            ['id' => 'bye', 'type' => 'leaveScene', 'characters' => ['catpope', 'plf'], 'excerpt' => null, 'group' => 'end', 'groupIndex' => 1],
            ['id' => 'long', 'type' => 'showMessage', 'characters' => [], 'excerpt' => str_repeat('a', 80) . '…', 'group' => 'end', 'groupIndex' => 1],
            ['id' => null, 'type' => 'jumpTo', 'characters' => [], 'excerpt' => null, 'group' => 'end', 'groupIndex' => 1],
        ], $events);
        self::assertSame([['id' => 'set1', 'type' => 'setVariable', 'characters' => [], 'excerpt' => null, 'group' => 'city', 'groupIndex' => 0]],
            $index['toons'][1]['chapters'][0]['events']);
        self::assertSame([['number' => 2, 'events' => [], 'choices' => []]], array_slice($index['toons'][1]['chapters'], 1));
        self::assertSame([], $catpope[0]['choices'], 'a chapter without a showChoices has no choices');
    }

    /** #550: a chapter's questions – each showChoices with its addChoices, whole labels by locale, from the published version */
    public function testTheChoicesOfAPublishedChapter(): void
    {
        $choice = fn(?string $id, array $label, array $extra = []) => ['type' => 'addChoice', 'params' => ['label' => $label]] + ($id !== null ? ['id' => $id] : []) + $extra;
        $published = $this->get(EntityStore::class)->insertPublished(EntityKey::parse('chapter:catpope:3'), (string) json_encode(['number' => 3, 'groups' => [
            ['label' => 'nave', 'slices' => [['events' => [
                // a slice's question; its "Only if" option is listed too
                $choice('stay', ['de' => 'Bleiben', 'en' => 'Stay']),
                $choice('go', ['de' => 'Gehen', 'en' => 'Go', 'fr' => ' '], ['if' => ['variable' => 'brave', 'op' => '==', 'value' => true]]),
                ['id' => 'ch3.door', 'type' => 'showChoices'],
            ]]], 'events' => [
                // a choice added, then the script leaves (an unconditional jumpTo): it belongs to no question
                $choice(null, ['de' => 'Verloren']),
                ['type' => 'jumpTo', 'if' => ['variable' => 'x', 'op' => '==', 'value' => 1], 'params' => ['target' => ['type' => 'chapter', 'value' => '4']]],
                ['type' => 'jumpTo', 'params' => ['target' => ['type' => 'chapter', 'value' => '4']]],
                // a question without an id: listed with `event` null
                $choice(null, ['en' => 'Yes']),
                $choice(null, ['en' => 'No']),
                ['type' => 'showChoices'],
            ]],
            ['slices' => [['items' => [['events' => [$choice('pet', ['de' => 'Streicheln']), ['id' => 'ch3.cat', 'type' => 'showChoices']]]]]]],
        ]], JSON_UNESCAPED_UNICODE), null);
        // a newer draft changes the labels and adds a question: events come from it, choices stay the published version's
        $this->get(EntityStore::class)->insert(EntityKey::parse('chapter:catpope:3'), (string) json_encode(['number' => 3, 'groups' => [
            ['label' => 'nave', 'events' => [$choice('stay', ['de' => 'Doch bleiben']), ['id' => 'ch3.draft', 'type' => 'showChoices']]],
        ]]), null, 'draft', null);
        // a chapter never published: no choices
        $this->get(EntityStore::class)->insert(EntityKey::parse('chapter:catpope:4'), (string) json_encode(['number' => 4, 'groups' => [
            ['events' => [$choice('a', ['en' => 'A']), ['id' => 'ch4.q', 'type' => 'showChoices']]],
        ]]), null, 'draft', null);

        $response = $this->request('/api/creator/story-events', $this->token('creator'));
        self::assertSame(200, $response->getStatusCode());
        $chapters = ((array) json_decode((string) $response->getBody(), true))['toons'][0]['chapters'];
        self::assertSame([3, 4], array_column($chapters, 'number'));
        self::assertSame(['stay', 'ch3.draft'], array_column($chapters[0]['events'], 'id'), 'the events are the newest version\'s');
        // per group: its script's questions, then its slices' (FlowCheck's order)
        self::assertSame([
            ['event' => null, 'group' => 'nave', 'options' => [
                ['id' => null, 'label' => ['en' => 'Yes']],
                ['id' => null, 'label' => ['en' => 'No']],
            ], 'version' => $published],
            ['event' => 'ch3.door', 'group' => 'nave', 'options' => [
                ['id' => 'stay', 'label' => ['de' => 'Bleiben', 'en' => 'Stay']],
                ['id' => 'go', 'label' => ['de' => 'Gehen', 'en' => 'Go']],
            ], 'version' => $published],
            ['event' => 'ch3.cat', 'group' => null, 'options' => [['id' => 'pet', 'label' => ['de' => 'Streicheln']]], 'version' => $published],
        ], $chapters[0]['choices']);
        self::assertSame([], $chapters[1]['choices'], 'never published');
    }
}
