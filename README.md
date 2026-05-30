# Mesh2Motion Desktop Application (WIP)

This is project that isn't fully functioning yet that allows an offline version application for Mesh2Motion. It uses the "Tauri" framework to build it (https://v2.tauri.app/). It is like electron, but creates much smaller binaries. 

The current limitations that need to be addressed are the following:
- Downloads don't work. Need to figure out how file system stuff works with Tauri
- Only have tried this on Windows. 
- Need updated icons
- Need to disable/hide things that break when the internet is not available
- probably more things once those issues are taken care of.

## Step 1: Prerequisites
- Git
- Node.js (v18 or higher recommended)
- Rust (https://rustup.rs/)

Note: If "rustc --version" isn't found, try to run this command to force powershell to refresh environment variables (Windows)

    $env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")


## Step 2: Clone the Mesh2Motion desktop repository with submodule (M2M app)

    git clone --recurse-submodules https://github.com/Mesh2Motion/mesh2motion-desktop.git
    cd mesh2motion-desktop

# 2. Install dependencies and build
    npm install
    npx tauri init
    npx tauri dev
    npm run build:desktop

Note: The first build will be very slow. This is because Rust needs to download and compile a bunch of components. Take a snack break and come back.

## Step 4: Find Your App

The built desktop application will be generated in the current folder:

- Windows: .exe 


