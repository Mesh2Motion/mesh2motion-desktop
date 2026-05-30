# scripts/build-desktop.ps1

Write-Host "Updating Mesh2Motion app submodule..." -ForegroundColor Cyan
git submodule update --init --recursive

Set-Location -Path "mesh2motion-app"

Write-Host "Pulling latest changes from main branch..." -ForegroundColor Cyan
git pull origin main

Write-Host "Installing dependencies..." -ForegroundColor Cyan
npm ci --prefer-offline

Write-Host "Building web app..." -ForegroundColor Cyan
npm run build

Set-Location -Path ".."

Write-Host "Building Desktop App with Pake..." -ForegroundColor Green
pake "./mesh2motion-app/dist/index.html" `
    --name "Mesh2Motion" `
    --width 1440 `
    --height 900 `
    --icon "./public/icon.png" `
    --enable-drag-drop `
    --hide-title-bar `
    --keep-binary

Write-Host "Build completed!" -ForegroundColor Green
Write-Host "Portable EXE should be available as: Mesh2Motion.exe" -ForegroundColor Green