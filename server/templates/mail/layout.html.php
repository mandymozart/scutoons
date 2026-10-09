<?php
/**
 * The HTML frame of every email (Domain\Mail\MailRenderer): ink on paper, monospace, square 2px frame, the SCUTOONS
 * header, the template's blocks, the fine print. Plain inline styles only (mail clients).
 * Blocks: p (a paragraph) · lines (a paragraph with line breaks) · button (a black link button) · code (a label + a big code)
 * · image (a picture as wide as the frame: url, text = its description – the newsletter).
 * @var callable $e  @var string $locale
 * @var list<array{type: string, text: string, url?: string, value?: string}> $blocks
 * @var list<string> $fineprint
 * @var array{text: string, url: string}|null $unsubscribe optional: a link under the fine print (the newsletter, Domain\Newsletter\NewsletterRenderer)
 */
$mono = "'IBM Plex Mono',Menlo,Consolas,monospace";
?>
<!doctype html>
<html lang="<?= $e($locale) ?>">
<body style="margin:0;padding:24px;background:#ffffff;color:#000000;font-family:<?= $mono ?>;font-size:15px;line-height:1.4">
<div style="max-width:480px;margin:0 auto;border:2px solid #000000;padding:24px">
<p style="margin:0 0 20px;font-weight:700;letter-spacing:1px">SCUTOONS</p>
<?php foreach ($blocks as $block): ?>
<?php if ($block['type'] === 'p'): ?>
<p style="margin:0 0 20px"><?= $e($block['text']) ?></p>
<?php elseif ($block['type'] === 'lines'): ?>
<p style="margin:0 0 20px"><?= implode('<br>', array_map($e, explode("
", $block['text']))) ?></p>
<?php elseif ($block['type'] === 'button'): ?>
<p style="margin:0 0 24px"><a href="<?= $e($block['url'] ?? '') ?>" style="display:inline-block;padding:12px 32px;background:#000000;color:#ffffff;text-decoration:none"><?= $e($block['text']) ?></a></p>
<?php elseif ($block['type'] === 'image'): ?>
<p style="margin:0 0 20px"><img src="<?= $e($block['url'] ?? '') ?>" alt="<?= $e($block['text']) ?>" width="480" style="display:block;width:100%;max-width:480px;height:auto;border:0"></p>
<?php elseif ($block['type'] === 'code'): ?>
<p style="margin:0 0 8px"><?= $e($block['text']) ?></p>
<p style="margin:0;font-size:28px;letter-spacing:6px;font-weight:700"><?= $e($block['value'] ?? '') ?></p>
<?php endif ?>
<?php endforeach ?>
<?php $unsubscribe ??= null; ?>
<?php if ($fineprint !== [] || $unsubscribe !== null): ?>
<p style="margin:24px 0 0;color:#6b6b6b;font-size:12px"><?= implode('<br>', array_map($e, $fineprint)) ?>
<?php if ($unsubscribe !== null): ?><?= $fineprint !== [] ? '<br>' : '' ?><a href="<?= $e($unsubscribe['url']) ?>" style="color:#6b6b6b"><?= $e($unsubscribe['text']) ?></a><?php endif ?></p>
<?php endif ?>
</div>
</body>
</html>
