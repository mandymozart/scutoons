<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Content\Reviews;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/** Review before publish (Domain\Content\Reviews, agents/CUTOVER.md 10.28 "Review levels"): rules, guard, submit, decide, mails */
final class ReviewsTest extends TestCase
{
    public function testWithoutRulesEveryCreatorPublishesDirectly(): void
    {
        $reviews = $this->get(Reviews::class);
        $writer = $this->user('writer@example.com');
        $reviews->guardPublish($writer, EntityKey::parse('chapter:catpope:2:de'), null);
        $reviews->setRule((string) $writer['id'], 'catpope', Reviews::NEEDS_REVIEW, true);
        $reviews->guardPublish($writer, EntityKey::parse('chapter:yellowmouse:2:de'), null); // another toon
        $reviews->guardPublish($writer, EntityKey::parse('characters'), null); // platform-wide: never reviewed
        foreach (['chapter:catpope:2:de', 'scene:catpope:2:kiosk:de', 'script:catpope:de', 'toon:catpope', 'cast:catpope'] as $key) {
            $this->assertApiError(403, 'review_required', fn() => $reviews->guardPublish($writer, EntityKey::parse($key), null));
        }
        $this->assertApiError(403, 'review_required', fn() => $reviews->guardPublish($writer, EntityKey::parse('chapter:catpope:2:de'), null, true));
        $reviews->setRule((string) $writer['id'], 'catpope', Reviews::NEEDS_REVIEW, false);
        $reviews->setRule((string) $writer['id'], Reviews::ALL_TOONS, Reviews::NEEDS_REVIEW, true);
        $this->assertApiError(403, 'review_required', fn() => $reviews->guardPublish($writer, EntityKey::parse('chapter:yellowmouse:2:de'), null));
        $this->assertApiError(400, 'invalid_toon', fn() => $reviews->setRule((string) $writer['id'], 'Cat Pope', Reviews::REVIEWER, true));
        $this->assertApiError(400, 'invalid_rule', fn() => $reviews->setRule((string) $writer['id'], 'catpope', 'publisher', true));
    }

    public function testSubmitPublishReturnAndMails(): void
    {
        $reviews = $this->get(Reviews::class);
        $store = $this->get(EntityStore::class);
        $writer = $this->user('writer@example.com');
        $editor = $this->user('editor@example.com');
        $other = $this->user('other@example.com');
        $reviews->setRule((string) $writer['id'], 'catpope', Reviews::NEEDS_REVIEW, true);
        $de = $store->save($writer, EntityKey::parse('chapter:catpope:2:de'), (object) ['content' => ['title' => 'Zwei']])['version']['id'];
        $en = $store->save($writer, EntityKey::parse('chapter:catpope:2:en'), (object) ['content' => ['title' => 'Two']])['version']['id'];
        $items = (object) ['items' => [(object) ['key' => 'chapter:catpope:2:de'], (object) ['key' => 'chapter:catpope:2:en']], 'note' => ' typo fixes '];

        $this->assertApiError(409, 'no_reviewer', fn() => $reviews->submit($writer, $items));
        $reviews->setRule((string) $writer['id'], 'catpope', Reviews::REVIEWER, true);
        $this->assertApiError(409, 'no_reviewer', fn() => $reviews->submit($writer, $items), 'the author never reviews themselves');
        $reviews->setRule((string) $editor['id'], 'catpope', Reviews::REVIEWER, true);
        $this->assertApiError(422, 'review_not_needed', fn() => $reviews->submit($writer, (object) ['items' => [(object) ['key' => 'characters']]]));
        $this->assertApiError(400, 'invalid_items', fn() => $reviews->submit($writer, (object) ['items' => []]));

        $submitted = $reviews->submit($writer, $items);
        self::assertSame([['chapter:catpope:2:de', $de, 'pending', 'typo fixes'], ['chapter:catpope:2:en', $en, 'pending', 'typo fixes']],
            array_map(fn(array $r) => [$r['key'], $r['version'], $r['state'], $r['note']], $submitted['reviews']));
        self::assertSame(['editor@example.com'], $submitted['reviewers']);
        self::assertSame(['review_requested'], $this->mails('editor@example.com'), 'one mail for both files');
        self::assertStringContainsString('chapter 2 (de, en)', $this->mailLog());

        // a newer submit of the same file annuls the open one; the review stays pinned to its version
        $de2 = $store->save($writer, EntityKey::parse('chapter:catpope:2:de'), (object) ['content' => ['title' => 'Zwei!']])['version']['id'];
        $again = $reviews->submit($writer, (object) ['items' => [(object) ['key' => 'chapter:catpope:2:de', 'version' => $de2]]])['reviews'][0];
        $de3 = $store->save($writer, EntityKey::parse('chapter:catpope:2:de'), (object) ['content' => ['title' => 'Zwei?']])['version']['id'];
        $overview = $reviews->overview($editor);
        self::assertSame([[$submitted['reviews'][1]['id'], $en, $en], [$again['id'], $de2, $de3]],
            array_map(fn(array $r) => [$r['id'], $r['version'], $r['newest']], $overview['to_review']));
        self::assertSame(['catpope'], $overview['reviewer_of']);
        self::assertSame(['annulled', 'pending', 'pending'], array_column(array_reverse($reviews->overview($writer)['mine']), 'state'));
        self::assertSame([], $reviews->overview($writer)['to_review'], 'own submits are not to review');

        // only a reviewer of the toon decides, never on their own submit
        $this->assertApiError(403, 'not_reviewer', fn() => $reviews->decide($other, (object) ['ids' => [$again['id']]], true));
        $this->assertApiError(403, 'not_reviewer', fn() => $reviews->decide($writer, (object) ['ids' => [$again['id']]], true));
        $this->assertApiError(409, 'review_closed', fn() => $reviews->decide($editor, (object) ['ids' => [$submitted['reviews'][0]['id']]], true));

        $published = $reviews->decide($editor, (object) ['ids' => [$again['id']], 'note' => 'Lovely'], true);
        self::assertSame(['chapter:catpope:2:de'], $published['keys']);
        self::assertSame($de2, (int) ($store->published('chapter:catpope:2:de')['id'] ?? 0), 'the submitted version, not the newer draft');
        $returned = $reviews->decide($editor, (object) ['ids' => [$submitted['reviews'][1]['id']], 'note' => 'Not yet'], false);
        self::assertSame(['returned', 'Not yet', 'editor@example.com'], [$returned['reviews'][0]['state'], $returned['reviews'][0]['decision_note'], $returned['reviews'][0]['decided_by']['email']]);
        self::assertNull($store->published('chapter:catpope:2:en'));
        self::assertSame(['review_published', 'review_returned'], $this->mails('writer@example.com'));
        $this->assertApiError(409, 'already_published', fn() => $reviews->submit($writer, (object) ['items' => [(object) ['key' => 'chapter:catpope:2:de', 'version' => $de2]]]));
    }

    public function testEveryToonReviewerAndThePlainPublishEndpoint(): void
    {
        $reviews = $this->get(Reviews::class);
        $store = $this->get(EntityStore::class);
        $writer = $this->user('writer@example.com');
        $boss = $this->user('boss@example.com');
        $reviews->setRule((string) $writer['id'], 'catpope', Reviews::NEEDS_REVIEW, true);
        $reviews->setRule((string) $boss['id'], Reviews::ALL_TOONS, Reviews::REVIEWER, true);
        $reviews->setRule((string) $boss['id'], Reviews::ALL_TOONS, Reviews::NEEDS_REVIEW, true); // a reviewer who needs review themselves
        $v = $store->save($writer, EntityKey::parse('season:catpope:1:de'), (object) ['content' => ['title' => 'S1']])['version']['id'];
        $review = $reviews->submit($writer, (object) ['items' => [(object) ['key' => 'season:catpope:1:de']]])['reviews'][0];
        self::assertSame([$review['id']], array_column($reviews->overview($boss)['to_review'], 'id'));
        $reviews->guardPublish($boss, EntityKey::parse('season:catpope:1:de'), $v); // the submitted version: allowed
        $this->assertApiError(403, 'review_required', fn() => $reviews->guardPublish($boss, EntityKey::parse('season:catpope:1:en'), null));
        $reviews->markPublished($boss, 'season:catpope:1:de', $v);
        self::assertSame('published', $reviews->overview($writer)['mine'][0]['state']);
    }

    public function testTheApiAppliesTheRulesToEveryCaller(): void
    {
        $writer = $this->user('writer@example.com');
        $editor = $this->user('editor@example.com');
        foreach ([$writer, $editor] as $u) $this->get(Roles::class)->grant((string) $u['id'], Roles::CREATOR);
        $reviews = $this->get(Reviews::class);
        $reviews->setRule((string) $writer['id'], 'catpope', Reviews::NEEDS_REVIEW, true);
        $reviews->setRule((string) $editor['id'], 'catpope', Reviews::REVIEWER, true);
        $asWriter = $this->bearer($writer);
        $asEditor = $this->bearer($editor);

        self::assertSame(200, $this->request('PUT', '/api/entities/chapter:catpope:2:de', $asWriter, '{"content":{"title":"Zwei"}}')->getStatusCode());
        $denied = $this->request('POST', '/api/entities/chapter:catpope:2:de/publish', $asWriter, '{}');
        self::assertSame([403, 'review_required', 'catpope'], [$denied->getStatusCode(), self::json($denied)['error'] ?? null,self::json($denied)['toon'] ?? null]);
        $submit = $this->request('POST', '/api/reviews', $asWriter, '{"items":[{"key":"chapter:catpope:2:de"}],"note":"please"}');
        self::assertSame(200, $submit->getStatusCode());
        $id = self::json($submit)['reviews'][0]['id'];
        $list = self::json($this->request('GET', '/api/reviews', $asEditor));
        self::assertSame([$id], array_column($list['to_review'], 'id'));
        self::assertSame(200, $this->request('POST', '/api/reviews/publish', $asEditor, json_encode(['ids' => [$id]]) ?: '')->getStatusCode());
        self::assertSame('published', self::json($this->request('GET', '/api/entities/chapter:catpope:2:de?wrap=1', $asWriter))['version']['status'] ?? null);
        self::assertSame(403, $this->request('POST', '/api/entities/chapter:catpope:2:de/unpublish', $asWriter, '{}')->getStatusCode());
        self::assertSame(200, $this->request('POST', '/api/entities/chapter:catpope:2:de/unpublish', $asEditor, '{}')->getStatusCode());
    }

    // ── Helpers ───────────────────────────────────────────────────────────────────────────────────────────

    /** @return list<string> */
    private function mails(string $to): array
    {
        return array_map('strval', array_column($this->db->all('SELECT template FROM mail_log WHERE recipient = ? ORDER BY id', [$to]), 'template'));
    }

    private function mailLog(): string
    {
        return (string) @file_get_contents($this->dir . '/mail.log');
    }

    private function assertApiError(int $status, string $code, callable $call, string $message = ''): void
    {
        try {
            $call();
            self::fail("ApiError $code expected" . ($message !== '' ? " ($message)" : ''));
        } catch (ApiError $error) {
            self::assertSame([$status, $code], [$error->status, $error->errorCode], $message);
        }
    }

    /**
     * @param array<string, mixed> $user
     * @return array<string, string>
     */
    private function bearer(array $user): array
    {
        return ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id']), 'Content-Type' => 'application/json'];
    }

    /** @param array<string, string> $headers */
    private function request(string $method, string $uri, array $headers = [], ?string $body = null): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        if ($body !== null) {
            $request->getBody()->write($body);
            $request->getBody()->rewind();
        }
        $container = Kernel::container(Settings::fixed(self::root(), $this->settings()));
        $container->set(Clock::class, $this->clock);
        return Kernel::app($container)->handle($request);
    }

    /** @return array<string, mixed> */
    private static function json(ResponseInterface $response): array
    {
        return (array) json_decode((string) $response->getBody(), true);
    }
}
