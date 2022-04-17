vk-rs
========================
This is a **hobby** project. The goal is to learn Rust and Vulkan.

Requirements
------------
* rustc

Usage
-----
Just use `cargo build`. But don't forget to compile shaders inside the `shaders` folder :

    glslc shader.vert -o vert.spv
    glslc shader.frag -o frag.spv


Debugging with VSCode & rust-analyser
-------------------------------------
The following task needs to be in your ```tasks.json``` file : 

    {
        "type": "cargo",
        "command": "build",
        "problemMatcher": [
            "$rustc"
        ],
        "group": {
            "kind": "build",
            "isDefault": true
        },
        "label": "rust: cargo build"
    }

The following configuration needs to be in your ```launch.json``` file :

    {
        "name": "vk-rs",
        "type": "cppvsdbg",
        "request": "launch",
        "program": "${workspaceFolder}/target/debug/vk-rs.exe",
        "args": [],
        "stopAtEntry": false,
        "cwd": "${workspaceFolder}",
        "environment": [],
        "console": "internalConsole",
        "internalConsoleOptions": "openOnSessionStart",
        "preLaunchTask": "rust: cargo build"
    }
