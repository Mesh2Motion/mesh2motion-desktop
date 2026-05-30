#!/bin/bash

# scripts/build-desktop.sh

set -e  # Exit on any error

echo "Updating Mesh2Motion app submodule..."
git submodule update --init --recursive

cd mesh2motion-app

echo "Pulling latest changes from main branch..."
git pull origin main

echo "Installing dependencies..."
npm ci --prefer-offline

echo "Building Mesh2Motion app..."
npm run build

# return to main desktop project directory
cd ..

echo "Building with Tauri with dev tools enabled..."
# replace with "npx tauri build" for normal (smaller) release build
npx tauri build -- --debug

echo "Build completed!"