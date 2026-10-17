fn main() {
    embed_windows_icon();
    // Slint's recursive UI compiler can exceed the Windows main-thread stack.
    // This affects build-time compilation only, not the application's threads.
    std::thread::Builder::new()
        .name("slint-ui-compiler".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            slint_build::compile("ui/app.slint").expect("failed to compile Slint UI");
        })
        .expect("failed to start Slint compiler")
        .join()
        .expect("Slint compiler thread failed");
}

fn embed_windows_icon() {
    println!("cargo:rerun-if-changed=ui/assets/branding/toolkit.ico");
    println!("cargo:rerun-if-changed=ui/assets/branding/toolkit-256.png");
    let target = std::env::var("TARGET").expect("Cargo target");
    if !target.contains("windows") {
        return;
    }

    let manifest = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let icon = manifest.join("ui/assets/branding/toolkit.ico");
    let rc = output.join("toolkit.rc");
    // Use an absolute forward-slash path: the SDK resource compiler accepts it
    // even when the checkout path contains spaces.
    std::fs::write(
        &rc,
        format!(
            "1 ICON \"{}\"\n",
            icon.display().to_string().replace('\\', "/")
        ),
    )
    .expect("write icon resource definition");
    let resource = output.join(if target.contains("msvc") {
        "toolkit.res"
    } else {
        "toolkit-icon.o"
    });
    let status = if target.contains("msvc") {
        let tool = cc::windows_registry::find_tool(&target, "rc.exe")
            .expect("Windows SDK resource compiler rc.exe is required");
        tool.to_command()
            .arg("/nologo")
            .arg("/fo")
            .arg(&resource)
            .arg(&rc)
            .status()
    } else {
        let compiler = std::env::var("WINDRES").unwrap_or_else(|_| "windres".into());
        std::process::Command::new(compiler)
            .arg("-i")
            .arg(&rc)
            .arg("-o")
            .arg(&resource)
            .arg("-O")
            .arg("coff")
            .status()
    }
    .expect("run Windows icon resource compiler");
    assert!(status.success(), "Windows icon resource compilation failed");
    println!(
        "cargo:rustc-link-arg-bin=tf2-demo-toolkit={}",
        resource.display()
    );
}
