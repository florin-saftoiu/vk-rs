use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=shaders");
    Command::new("glslc")
        .args(&["shaders/shader.frag", "-o", "shaders/frag.spv"])
        .status()
        .unwrap();
    Command::new("glslc")
        .args(&["shaders/shader.vert", "-o", "shaders/vert.spv"])
        .status()
        .unwrap();
}
