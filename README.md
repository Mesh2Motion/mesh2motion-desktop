# Build Desktop Application (Using Pake)

Prerequisites
- Git
- Node.js (v18 or higher recommended)


## Step 2: Install Pake CLI

Pake  
    npm install -g pake-cli

test installation: pake --version

## Step 3: Clone the Mesh2Motion desktop repository with submodule (M2M app)

    git clone --recurse-submodules https://github.com/Mesh2Motion/mesh2motion-desktop.git
    cd mesh2motion-desktop

# 2. Install dependencies and build
    npm install
    npm run build:desktop

## Step 4: Find Your App

The built desktop application will be generated in the current folder:

- macOS: .dmg installer + .app
- Windows: .exe installer
- Linux: .deb package or AppImage

