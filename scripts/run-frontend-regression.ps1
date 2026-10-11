param([int]$Port = 1422)
$ErrorActionPreference = 'Stop'
$projectPath = Split-Path -Parent $PSScriptRoot
$nodePath = (Get-Command node -ErrorAction Stop).Source
$vitePath = Join-Path $projectPath 'node_modules\vite\bin\vite.js'
$testPath = Join-Path $PSScriptRoot 'frontend-regression.cjs'
$portProbe = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, $Port)
try { $portProbe.Start() } finally { $portProbe.Stop() }
$testServer = $null
try {
    $testServer = Start-Process -FilePath $nodePath -ArgumentList @($vitePath, '--mode', 'tauri', '--host', '127.0.0.1', '--port', $Port) -WorkingDirectory $projectPath -WindowStyle Hidden -PassThru
    $env:FRONTEND_TEST_URL = "http://127.0.0.1:$Port"
    $ready = $false
    for ($attempt = 0; $attempt -lt 20; $attempt++) {
        if ($testServer.HasExited) { throw 'Test Vite server exited before becoming ready' }
        try {
            $response = Invoke-WebRequest -Uri $env:FRONTEND_TEST_URL -UseBasicParsing -TimeoutSec 1
            if ($response.StatusCode -eq 200) { $ready = $true; break }
        } catch { Start-Sleep -Milliseconds 250 }
    }
    if (!$ready) { throw 'Test Vite server did not become ready' }
    Push-Location $projectPath
    try {
        & $nodePath $testPath
        if ($LASTEXITCODE -ne 0) { throw "Frontend regression failed: exit $LASTEXITCODE" }
    } finally { Pop-Location }
} finally {
    if ($testServer -and !$testServer.HasExited) { Stop-Process -Id $testServer.Id -ErrorAction SilentlyContinue }
}
