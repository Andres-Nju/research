fn up_to_date(config: &Config, testpaths: &TestPaths, props: &EarlyProps) -> bool {
    let rust_src_dir = config.find_rust_src_root().expect(
        "Could not find Rust source root",
    );
    let stamp = mtime(&stamp(config, testpaths));
    let mut inputs = vec![mtime(&testpaths.file), mtime(&config.rustc_path)];
    for aux in props.aux.iter() {
        inputs.push(mtime(
            &testpaths.file.parent().unwrap().join("auxiliary").join(
                aux,
            ),
        ));
    }
    // Relevant pretty printer files
    let pretty_printer_files = [
        "src/etc/debugger_pretty_printers_common.py",
        "src/etc/gdb_load_rust_pretty_printers.py",
        "src/etc/gdb_rust_pretty_printing.py",
        "src/etc/lldb_batchmode.py",
        "src/etc/lldb_rust_formatters.py",
    ];
    for pretty_printer_file in &pretty_printer_files {
        inputs.push(mtime(&rust_src_dir.join(pretty_printer_file)));
    }
    for lib in config.run_lib_path.read_dir().unwrap() {
        let lib = lib.unwrap();
        inputs.push(mtime(&lib.path()));
    }
    inputs.iter().any(|input| *input > stamp)
}
