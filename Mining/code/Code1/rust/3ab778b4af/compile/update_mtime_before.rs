fn update_mtime(path: &Path) {
    let mut max = None;
    if let Ok(entries) = path.parent().unwrap().read_dir() {
        for entry in entries.map(|e| t!(e)) {
            if t!(entry.file_type()).is_file() {
                let meta = t!(entry.metadata());
                let time = FileTime::from_last_modification_time(&meta);
                max = cmp::max(max, Some(time));
            }
        }
    }

    if !max.is_none() && max <= Some(mtime(path)) {
        return
    }
    t!(File::create(path));
}
