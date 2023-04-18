fn open_docs(path: &Path) -> Result<&'static str, Vec<&'static str>> {
    match Command::new("cmd").arg("/C").arg("start").arg("").arg(path).status() {
        Ok(_) => return Ok("cmd /C start"),
        Err(_) => return Err(vec!["cmd /C start"])
    };
}
