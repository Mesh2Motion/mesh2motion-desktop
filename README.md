# Mesh2Motion Desktop Application (WIP)

This is project that isn't fully functioning yet that allows an offline version application for Mesh2Motion. It uses the "Tauri" framework to build it (https://v2.tauri.app/). It is like electron, but creates much smaller binaries. 

The current limitations that need to be addressed are the following:
- Downloading animations automatically goes to "Downloads" folder. Should change this to a 'save as' dialog
- Currently outputting the development build, so it has dev tools available (Ctrl+Shift+I)
- probably more things once those issues are taken care of.

## Step 1: Prerequisites
- Git
- Node.js (v22 or higher recommended)
- Rust (https://rustup.rs/). Use standard installation process

If on Windows, you need the Visual Studio Build Tools for C++. During the process make sure this component is installed when given an option:
"Desktop Development with C++". This should contain the build tools like a linker as well as the Windows SDK. You will need to restart Windows after for the tools to all be operable.

https://visualstudio.microsoft.com/visual-cpp-build-tools/


Testing if Rust is installed: 
    rustc --version

If the command isn't found and you are using visual studio code, close down visual studio code and restart. That helped me.


## Step 2: Clone the Mesh2Motion desktop repository with submodule (M2M app)

    git clone --recurse-submodules https://github.com/Mesh2Motion/mesh2motion-desktop.git
    cd mesh2motion-desktop

# 2. Install dependencies and build
    npm install

**if on windows**

    npm run build:windows   

**if on linux/MacOS**

    npm run build:linux   

Note: The first build will be very slow. This is because Rust needs to download and compile a bunch of components. Take a snack break and come back.

## Step 4: Find Your App

The built desktop application will be generated in the folder:

(release build)
src-tauri > target > release > tauri-app.exe

(debug build that has developer tools enabled - default for now)
src-tauri > target > debug > tauri-app.exe



# Other 

When generating the application icon, you don't need to create all the versions. Update the "app-icon.png". Then run the following command

    npm run tauri icon
