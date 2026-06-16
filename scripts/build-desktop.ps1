# scripts/build-desktop.ps1

Write-Host "Updating Mesh2Motion app submodule..." -ForegroundColor Cyan
git submodule update --init --recursive

Set-Location -Path "mesh2motion-app"

Write-Host "Pulling latest changes from main branch..." -ForegroundColor Cyan
git pull origin main

Write-Host "Installing dependencies..." -ForegroundColor Cyan
npm ci --prefer-offline

Write-Host "Building Mesh2Motion app..." -ForegroundColor Cyan
npm run build

# return to main desktop project directory
Set-Location -Path ".."

Write-Host "Building Tauri application..." -ForegroundColor Green
# replace with "npx tauri build" for normal (smaller) release build 
# replace with "npx tauri build -- --debug" for development build with dev tools enabled (larger)
npx tauri build

Write-Host "Build completed!" -ForegroundColor Green