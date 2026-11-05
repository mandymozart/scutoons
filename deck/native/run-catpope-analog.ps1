# Starts the catpope-analog scene (the being with the analog synth and the 16-step sequencer, #418) on this PC.
# It sets DECK_MODULES for THIS process, so nothing has to be set in the shell first. The deck loads the module wasm files in
# that folder (deck/target/hear-modules: analog_module.wasm and stepseq_module.wasm) and the console must print a line
#   module 'analog' 0.1.0 (91 params) from ... compiled in ...
# and one for 'stepseq'. If those lines are missing, the deck has no module library and F5 says "unknown kind module:stepseq".
# Optional: pass more deck flags, e.g.  .\run-catpope-analog.ps1 --audio-device Fireface
# (ASCII only: Windows PowerShell 5.1 reads a script without a BOM as ANSI)
$deck = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$modules = Join-Path $deck 'target\hear-modules'
$exe = Join-Path $deck 'target\debug\deck.exe'
if (-not (Test-Path (Join-Path $modules 'analog_module.wasm'))) {
    Write-Error "missing analog_module.wasm in $modules - build both modules (cargo build --release --target wasm32-unknown-unknown -p analog-module -p stepseq-module --target-dir target/wasm-modules) and copy analog_module.wasm and stepseq_module.wasm there"
    exit 1
}
$env:DECK_MODULES = $modules
$env:RUST_LOG = 'info'
# --audio-host wasapi: this build has no ASIO (cargo build --features asio needs Steinberg's ASIO SDK); the deck otherwise says
# "Cpal not available (this build has no ASIO)" and falls back to NO audio when the config asks for ASIO. Later flags win.
& $exe --scene catpope-analog --windowed --audio-host wasapi @args
