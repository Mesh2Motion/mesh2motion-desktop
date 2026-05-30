# Build Desktop Application (Using Pake)

Prerequisites
- Git
- Node.js (v18 or higher recommended)


## Step 1: Clone the Mesh2Motion Repository and build project

    git clone https://github.com/Mesh2Motion/mesh2motion-app.git
    cd mesh2motion-app
    npm install
    npm run build

## Step 2: Install Pake CLI

Pake  
    npm install -g pake-cli

test installation: pake --version

## Step 3: Create the Desktop App (currently in mesh2motion repository)
pake ./dist/index.html \
  --name "Mesh2Motion" \
  --width 1440 \
  --height 900 \
  --icon ./public/logo.png \     # Replace with your actual icon path
  --enable-drag-drop \
  --resizable


## Step 4: Find Your App

The built desktop application will be generated in the current folder:

- macOS: .dmg installer + .app
- Windows: .exe installer
- Linux: .deb package or AppImage





git submodule add https://github.com/Mesh2Motion/mesh2motion-app.git
git commit -m "Add mesh2motion-web as submodule"