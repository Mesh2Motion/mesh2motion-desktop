# scripts/build-desktop.ps1

Write-Host "Updating Mesh2Motion web submodule..." -ForegroundColor Cyan
git submodule update --init --recursive

Set-Location -Path "mesh2motion-web"

Write-Host "Pulling latest changes from main branch..." -ForegroundColor Cyan
git pull origin main

Write-Host "Installing dependencies..." -ForegroundColor Cyan
pnpm install --frozen-lockfile

Write-Host "Building web app..." -ForegroundColor Cyan
npm run build

Set-Location -Path ".."

Write-Host "Building Desktop App with Pake..." -ForegroundColor Green
pake "./mesh2motion-web/dist/index.html" `
    --name "Mesh2Motion" `
    --width 1440 `
    --height 900 `
    --icon "./public/icon.png" `
    --enable-drag-drop `
    --resizable