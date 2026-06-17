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


# Doing a release build (WIP)

All the final binaries are made with github actions. These spin up build agents that do the work. The .github/workflows/release.yml configures this.
This process should automatically run since the repository is in Github. A new build kicks off when a version tag is added and pushed to the repository


The process for doing a new release should be as follows:

1. Update the src/tauri/tauri.conf.json and src-tauri/cargo.toml to new version
2. Add the files to staging in GIT: git add .
3. Create a commit in GIT similar to this: git commit -m "chore: release v0.1.0"
4. Create tag for release: git tag v0.1.0
5. push tag to Github: git push origin main --tags


Github Actions. If you go onto Github.com project, there is an "Actions" tab where you can see the work it does.

### potential errors
You might get an error building with Github actions like this "fatal: refusing to merge unrelated histories"
This means submodule (M2M web app) is old on your computer. Go into the mesh2motion-app folder and make sure you are on the "main" branch

    cd mesh2motion-app
    git checkout main

Then do the following to get the latest and get back to the project root:

    git pull --rebase
    cd ../

Push those changes out and try to kick off another build.


# Other 

When generating the application icon, you don't need to create all the versions. Update the "app-icon.png". Then run the following command

    npm run tauri icon

If you decide to update Node.js, we need to make sure to update the .github workflow to also use that.
