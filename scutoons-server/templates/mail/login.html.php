<?php
/**
 * HTML part of the login mail. Design: ink on paper, monospace, square 2px frame, black button.
 * @var callable $e  @var string $locale  @var string $link  @var string $otp  @var bool $signup  @var ?string $claimCode
 * @var callable $t  text by key (placeholders filled)
 */
$mono = "'IBM Plex Mono',Menlo,Consolas,monospace";
?>
<!doctype html>
<html lang="<?= $e($locale) ?>">
<body style="margin:0;padding:24px;background:#ffffff;color:#000000;font-family:<?= $mono ?>;font-size:15px;line-height:1.4">
<div style="max-width:480px;margin:0 auto;border:2px solid #000000;padding:24px">
<p style="margin:0 0 20px;font-weight:700;letter-spacing:1px">SCUTOONS</p>
<p style="margin:0 0 20px"><?= $e($t('intro')) ?></p>
<?php if ($signup): ?>
<p style="margin:0 0 20px"><?= $e($t('signup')) ?></p>
<?php endif ?>
<?php if ($claimCode !== null): ?>
<p style="margin:0 0 20px"><?= $e($t('claim')) ?></p>
<?php endif ?>
<p style="margin:0 0 24px"><a href="<?= $e($link) ?>" style="display:inline-block;padding:12px 32px;background:#000000;color:#ffffff;text-decoration:none"><?= $e($t('button')) ?></a></p>
<p style="margin:0 0 8px"><?= $e($t('otp')) ?></p>
<p style="margin:0;font-size:28px;letter-spacing:6px;font-weight:700"><?= $e($otp) ?></p>
<p style="margin:24px 0 0;color:#6b6b6b;font-size:12px"><?= $e($t('validity')) ?><br><?= $e($t('ignore')) ?></p>
</div>
</body>
</html>
