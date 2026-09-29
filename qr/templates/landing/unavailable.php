<?php
/**
 * The API does not answer: the scan itself worked. Continue when this code was looked up recently (cache).
 * @var callable $e  @var array<string, string> $t  @var ?string $continueUrl
 */
?>
<div class="panel" id="unavailable"><h1><?= $e($t['unavailable']) ?></h1><p><?= $e($t['unavailable_hint']) ?></p>
<div class="actions">
<?php if ($continueUrl !== null): ?>
<a class="btn" id="continue" href="<?= $e($continueUrl) ?>"><?= $e($t['continue']) ?></a><p class="caption"><?= $e($t['continue_hint']) ?></p>
<?php endif ?>
<a class="btn btn-secondary" id="retry" href=""><?= $e($t['retry']) ?></a>
</div></div>
