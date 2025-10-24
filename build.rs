//! This build script copies the `memory.x` file from the crate root into
//! a directory where the linker can always find it at build time.
//! For many projects this is optional, as the linker always searches the
//! project root directory -- wherever `Cargo.toml` is. However, if you
//! are using a workspace or have a more complicated build setup, this
//! build script becomes required. Additionally, by requesting that
//! Cargo re-run the build script whenever `memory.x` is changed,
//! updating `memory.x` ensures a rebuild of the application with the
//! new memory settings.

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use std::env;

fn main() {
    // Generate bindings first so our rust code has access to the C code
    generate_bindings();

    // Get wifi credentials from .env file
    load_wifi_credentials();

    // Put the linker script somewhere the linker can find it
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    println!("cargo:rustc-link-search={}", out.display());

    // The file `memory.x` is loaded by cortex-m-rt's `link.x` script, which
    // is what we specify in `.cargo/config.toml` for Arm builds
    let memory_x = include_bytes!("memory.x");
    let mut f = File::create(out.join("memory.x")).unwrap();
    f.write_all(memory_x).unwrap();
    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rerun-if-changed=.env");
    println!("cargo:rerun-if-changed=build.rs");

    // As last step we compile the C code and link everything together
    compile_and_link_debugprobe();
}

fn load_wifi_credentials() {
    // Load .env file and generate WiFi config
    if let Err(e) = dotenv::dotenv() {
        println!("cargo:warning=Could not load .env file: {}", e);
    }

    // Generate WiFi configuration file
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let wifi_ssid = std::env::var("WIFI_SSID").unwrap_or_else(|_| "DefaultSSID".to_string());
    let wifi_password =
        std::env::var("WIFI_PASSWORD").unwrap_or_else(|_| "DefaultPassword".to_string());

    let wifi_config = format!(
        r#"#[allow(dead_code)]
pub const WIFI_SSID: &str = "{}";
#[allow(dead_code)]
pub const WIFI_PASSWORD: &str = "{}";
"#,
        wifi_ssid, wifi_password
    );

    let mut f = File::create(out.join("wifi_config.rs")).unwrap();
    f.write_all(wifi_config.as_bytes()).unwrap();
}

fn compile_and_link_debugprobe() {
    // Get the directory where build.rs is located
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let debugprobe_dir = PathBuf::from(&manifest_dir).join("debugprobe");

    // build.rs
    cc::Build::new()
        .file(debugprobe_dir.join("CMSIS_DAP/CMSIS/DAP/Firmware/Source/DAP.c"))
        .file(debugprobe_dir.join("CMSIS_DAP/CMSIS/DAP/Firmware/Source/JTAG_DP.c"))
        .file(debugprobe_dir.join("CMSIS_DAP/CMSIS/DAP/Firmware/Source/DAP_vendor.c"))
        .file(debugprobe_dir.join("CMSIS_DAP/CMSIS/DAP/Firmware/Source/SWO.c"))
        // .file(debugprobe_dir.join("src/sw_dp_pio.c")) // RP PIO version of CMSIS_DAP/CMSIS/DAP/Firmware/Source/SW_DP.c
        .include(debugprobe_dir.join("CMSIS_DAP/CMSIS/DAP/Firmware/Include/"))
        .include(debugprobe_dir.join("CMSIS_DAP/CMSIS/Core/Include/"))
        .include(debugprobe_dir.join("include/"))
        .include(debugprobe_dir.join("src/"))
        .include("csrc/")
        .compile("debugprobe");

    println!("cargo:rustc-link-lib=static=debugprobe");
}

fn generate_bindings() {
    // Get the directory where build.rs is located
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let debugprobe_dir = PathBuf::from(&manifest_dir).join("debugprobe");

    let bindings = bindgen::Builder::default()
        .detect_include_paths(true)
        .header(
            debugprobe_dir
                .join("CMSIS_DAP/CMSIS/DAP/Firmware/Include/DAP.h")
                .to_str()
                .unwrap(),
        )
        .header(
            debugprobe_dir
                .join("include/DAP_config.h")
                .to_str()
                .unwrap(),
        )
        .clang_arg(format!("-I{}", debugprobe_dir.join("src").display()))
        .clang_arg(format!(
            "-I{}",
            debugprobe_dir
                .join("CMSIS_DAP/CMSIS/Core/Include")
                .display()
        ))
        .clang_arg(format!(
            "-I{}",
            debugprobe_dir
                .join("CMSIS_DAP/CMSIS/DAP/Firmware/Include")
                .display()
        ))
        .use_core()
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let bindings_path = out_path.join("bindings.rs");
    bindings
        .write_to_file(bindings_path)
        .expect("Couldn't write bindings!");
}
