use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=packaging/emulator-81.ico");
    println!("cargo:rerun-if-env-changed=WINDRES");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let icon =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("packaging/emulator-81.ico");
    let icon = icon
        .to_str()
        .expect("Windows icon path must be valid Unicode")
        .replace('\\', "/")
        .replace('"', "\\\"");
    let output_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let source = output_dir.join("emulator-81.rc");
    fs::write(&source, format!("1 ICON \"{icon}\"\n")).expect("write Windows icon resource");

    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap();
    let output = output_dir.join(if target_env == "msvc" {
        "emulator-81.res"
    } else {
        "emulator-81.o"
    });
    let status = match target_env.as_str() {
        "gnu" => {
            let target = env::var("TARGET").unwrap();
            let prefix = target
                .strip_suffix("-pc-windows-gnu")
                .expect("unsupported Windows GNU target");
            let windres = env::var_os("WINDRES")
                .unwrap_or_else(|| format!("{prefix}-w64-mingw32-windres").into());
            Command::new(windres)
                .arg("-i")
                .arg(&source)
                .arg("-o")
                .arg(&output)
                .args(["-O", "coff"])
                .status()
        }
        "msvc" => Command::new("rc.exe")
            .arg("/nologo")
            .arg(format!("/fo{}", output.display()))
            .arg(&source)
            .status(),
        other => panic!("unsupported Windows target environment: {other}"),
    }
    .expect("run Windows resource compiler");
    assert!(
        status.success(),
        "Windows resource compiler failed: {status}"
    );
    println!("cargo:rustc-link-arg={}", output.display());
}
