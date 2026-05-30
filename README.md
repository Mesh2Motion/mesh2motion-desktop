# Build Desktop Application (Using Pake)

## Step 1: Prerequisites
- Git
- Node.js (v18 or higher recommended)
- Rust (https://rustup.rs/). After running with prompt, install standard installation.

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

- Windows: .exe portable runner


