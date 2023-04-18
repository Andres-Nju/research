fn main() {
    let start = Instant::now();

    // This must use the Ninja generator -- it's the only one that
    // parallelizes cmake's output properly.  (Cmake generates
    // separate makefiles, each of which try to build
    // ParserResults.pkl, and then stomp on eachother.)
    let mut build = cmake::Config::new(".");

    let target = env::var("TARGET").unwrap();
    if target.contains("windows-msvc") {
        // We must use Ninja on Windows for this -- msbuild is painfully slow,
        // and ninja is easier to install than make.
        build.generator("Ninja");
        // because we're using ninja, we need to explicitly set these
        // to VC++, otherwise it'll try to use cc
        build.define("CMAKE_C_COMPILER", "cl.exe")
             .define("CMAKE_CXX_COMPILER", "cl.exe");
        // We have to explicitly specify the full path to link.exe,
        // for reasons that I don't understand.  If we just give
        // link.exe, it tries to use script-*/out/link.exe, which of
        // course does not exist.
        let link = std::process::Command::new("where").arg("link.exe").output().unwrap();
        let link_path: Vec<&str> = std::str::from_utf8(&link.stdout).unwrap().split("\r\n").collect();
        build.define("CMAKE_LINKER", link_path[0]);
    }

    build.build();

    println!("Binding generation completed in {}s", start.elapsed().as_secs());

    let json = PathBuf::from(env::var("OUT_DIR").unwrap()).join("build").join("InterfaceObjectMapData.json");
    let json: Value = serde_json::from_reader(File::open(&json).unwrap()).unwrap();
    let mut map = phf_codegen::Map::new();
    for (key, value) in json.as_object().unwrap() {
        map.entry(Bytes(key), value.as_str().unwrap());
    }
    let phf = PathBuf::from(env::var("OUT_DIR").unwrap()).join("InterfaceObjectMapPhf.rs");
    let mut phf = File::create(&phf).unwrap();
    write!(&mut phf, "pub static MAP: phf::Map<&'static [u8], unsafe fn(*mut JSContext, HandleObject)> = ").unwrap();
    map.build(&mut phf).unwrap();
    write!(&mut phf, ";\n").unwrap();
}
