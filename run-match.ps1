$ErrorActionPreference = 'Stop'

$projectRoot = $PSScriptRoot
$engineDirectory = Join-Path $projectRoot 'docker_image'
$solutionDirectory = (Resolve-Path (Join-Path $projectRoot 'solution')).Path

if (-not (Test-Path (Join-Path $engineDirectory 'linux_game_engine'))) {
    throw 'Missing docker_image/linux_game_engine. Extract filler.zip into the project root.'
}

docker --context desktop-linux build --tag filler-engine $engineDirectory
if ($LASTEXITCODE -ne 0) {
    throw "Docker image build failed with exit code $LASTEXITCODE."
}

$mount = "type=bind,source=$solutionDirectory,target=/filler/solution"
docker --context desktop-linux run --rm --mount $mount filler-engine /filler/run-matches.sh
if ($LASTEXITCODE -ne 0) {
    throw "Matches failed with exit code $LASTEXITCODE. See Docker output above."
}
