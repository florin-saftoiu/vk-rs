use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=shaders");
    assert!(Command::new("glslc")
        .args(&["shaders/shader.vert", "-o", "shaders/vert.spv"])
        .status()
        .unwrap()
        .success());
    assert!(Command::new("glslc")
        .args(&["shaders/shader.frag", "-o", "shaders/frag.spv"])
        .status()
        .unwrap()
        .success());
}
