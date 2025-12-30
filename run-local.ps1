# run-local.ps1
# This script spawns 3 instances of the raft-kv service locally.

$env:PATH = "D:\projects\mingw64\bin;" + $env:PATH
$exe = "target\release\raft-kv.exe"

if (-not (Test-Path $exe)) {
    Write-Host "Error: target\release\raft-kv.exe not found. Please run 'cargo build --release' first." -ForegroundColor Red
    exit
}

# Close old instances
Get-Process -Name raft-kv -ErrorAction SilentlyContinue | Stop-Process -Force

$ports = @(
    @{ http = 8081; grpc = 50051; id = "node-1"; is_leader = $true };
    @{ http = 8082; grpc = 50052; id = "node-2"; is_leader = $false };
    @{ http = 8083; grpc = 50053; id = "node-3"; is_leader = $false }
)

$peers = "http://127.0.0.1:50051,http://127.0.0.1:50052,http://127.0.0.1:50053"

foreach ($p in $ports) {
    Write-Host "Starting $($p.id) (HTTP: $($p.http), gRPC: $($p.grpc))..." -ForegroundColor Cyan
    
    $args = @("--http-port", $p.http, "--grpc-port", $p.grpc, "--node-id", $p.id, "--peers", $peers)
    if ($p.is_leader) {
        $args += "--is-leader"
    }

    Start-Process -FilePath $exe -ArgumentList $args -NoNewWindow
}

Write-Host "`nCluster started!" -ForegroundColor Green
Write-Host "Node-1 (Leader): http://localhost:8081"
Write-Host "Node-2 (Follower): http://localhost:8082"
Write-Host "Node-3 (Follower): http://localhost:8083"

Write-Host "`n--- Testing Commands (PowerShell) ---" -ForegroundColor Yellow
Write-Host "Set a key:    Invoke-RestMethod -Method Post -Uri 'http://localhost:8081/mykey' -Body '{\`"value\`": \`"hello\`"}' -ContentType 'application/json'"
Write-Host "Get a key:    Invoke-RestMethod -Uri 'http://localhost:8082/mykey'"

Write-Host "`n--- Testing Commands (Standard cURL) ---" -ForegroundColor Yellow
Write-Host "Set a key:    curl.exe -X POST -H 'Content-Type: application/json' -d '{\`"value\`": \`"hello\`"}' http://localhost:8081/mykey"
Write-Host "Get a key:    curl.exe http://localhost:8082/mykey"
