<?php
/**
 * HTML part of the "no account yet" mail (no link to sign in, no one-time code).
 * @var callable $e  @var string $locale  @var string $viewerUrl  @var list<string> $paragraphs  @var callable $t
 */
$mono = "'IBM Plex Mono',Menlo,Consolas,monospace";
?>
<!doctype html>
<html lang="<?= $e($locale) ?>">
<body style="margin:0;padding:24px;background:#ffffff;color:#000000;font-family:<?= $mono ?>;font-size:15px;line-height:1.4">
<div style="max-width:480px;margin:0 auto;border:2px solid #000000;padding:24px">
<p style="margin:0 0 20px;font-weight:700;letter-spacing:1px">SCUTOONS</p>
<?php foreach ($paragraphs as $paragraph): ?>
<p style="margin:0 0 20px"><?= $e($paragraph) ?></p>
<?php endforeach ?>
<p style="margin:0 0 24px"><a href="<?= $e($viewerUrl) ?>" style="display:inline-block;padding:12px 32px;background:#000000;color:#ffffff;text-decoration:none"><?= $e($t('na_button')) ?></a></p>
<p style="margin:24px 0 0;color:#6b6b6b;font-size:12px"><?= $e($t('na_ignore')) ?></p>
</div>
</body>
</html>
