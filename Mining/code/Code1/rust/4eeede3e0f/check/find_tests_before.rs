fn find_tests(dir: &Path,
              target: &str,
              dst: &mut Vec<PathBuf>) {
    for e in t!(dir.read_dir()).map(|e| t!(e)) {
        let file_type = t!(e.file_type());
        if !file_type.is_file() {
            continue
        }
        let filename = e.file_name().into_string().unwrap();
        if (target.contains("windows") && filename.ends_with(".exe")) ||
           (!target.contains("windows") && !filename.contains(".")) ||
           (target.contains("emscripten") && filename.contains(".js")){
            dst.push(e.path());
        }
    }
}
