fn open_docs(path: &Path) -> Result<&'static str, Vec<&'static str>> {
    match Command::new("cmd").arg("/C").arg(path).status() {
        Ok(_) => return Ok("cmd /C"),
        Err(_) => return Err(vec!["cmd /C"])
    };
}
