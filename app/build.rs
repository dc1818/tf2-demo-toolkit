fn main() {
    // Slint's recursive UI compiler can exceed the Windows main-thread stack.
    // This affects build-time compilation only, not the application's threads.
    std::thread::Builder::new()
        .name("slint-ui-compiler".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            slint_build::compile("ui/app.slint")
                .expect("failed to compile Slint UI");
        })
        .expect("failed to start Slint compiler")
        .join()
        .expect("Slint compiler thread failed");
}
