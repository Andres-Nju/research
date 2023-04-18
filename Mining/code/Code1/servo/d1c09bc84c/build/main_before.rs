fn main() {
    let python = match env::var("PYTHON") {
        Ok(python_path) => python_path,
        Err(_) => find_python(),
    };
    let style = Path::new(file!()).parent().unwrap();
    let mako = style.join("Mako-0.9.1.zip");
    let template = style.join("properties.mako.rs");
    let product = if cfg!(feature = "gecko") { "gecko" } else { "servo" };
    let result = Command::new(python)
        .env("PYTHONPATH", &mako)
        .env("TEMPLATE", &template)
        .env("PRODUCT", product)
        .arg("-c")
        .arg(r#"
import os
import sys
from mako.template import Template
from mako import exceptions
try:
    print(Template(filename=os.environ['TEMPLATE'], input_encoding='utf8').render(PRODUCT=os.environ['PRODUCT'])
                                                                          .encode('utf8'))
except:
    sys.stderr.write(exceptions.text_error_template().render().encode('utf8'))
    sys.exit(1)
"#)
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    if !result.status.success() {
        exit(1)
    }
    let out = env::var("OUT_DIR").unwrap();
    File::create(&Path::new(&out).join("properties.rs")).unwrap().write_all(&result.stdout).unwrap();
}
