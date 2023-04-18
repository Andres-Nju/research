fn properties_list_json() {
    let top = Path::new(file!()).parent().unwrap().join("..").join("..").join("..").join("..");
    let json = top.join("target").join("doc").join("servo").join("css-properties.json");
    if json.exists() {
        remove_file(&json).unwrap()
    }
    let python = env::var("PYTHON").ok().unwrap_or_else(find_python);
    let script = top.join("components").join("style").join("properties").join("build.py");
    let status = Command::new(python)
        .arg(&script)
        .arg("servo")
        .arg("html")
        .arg("regular")
        .status()
        .unwrap();
    assert!(status.success());

    let properties: Value = serde_json::from_reader(File::open(json).unwrap()).unwrap();
    assert!(properties.as_object().unwrap().len() > 100);
    assert!(properties.as_object().unwrap().contains_key("margin"));
    assert!(properties.as_object().unwrap().contains_key("margin-top"));
}
