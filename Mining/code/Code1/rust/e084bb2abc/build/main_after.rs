fn main() {
    let target = env::var("TARGET").expect("TARGET was not set");
    let host = env::var("HOST").expect("HOST was not set");
    if cfg!(feature = "backtrace") && !target.contains("apple") && !target.contains("msvc") &&
        !target.contains("emscripten") && !target.contains("fuchsia") && !target.contains("redox") {
        let _ = build_libbacktrace(&host, &target);
    }

    if target.contains("linux") {
        if target.contains("android") {
            println!("cargo:rustc-link-lib=dl");
            println!("cargo:rustc-link-lib=log");
            println!("cargo:rustc-link-lib=gcc");
        } else if !target.contains("musl") || target.contains("mips") {
            println!("cargo:rustc-link-lib=dl");
            println!("cargo:rustc-link-lib=rt");
            println!("cargo:rustc-link-lib=pthread");
        }
    } else if target.contains("freebsd") {
        println!("cargo:rustc-link-lib=execinfo");
        println!("cargo:rustc-link-lib=pthread");
    } else if target.contains("dragonfly") || target.contains("bitrig") ||
              target.contains("netbsd") || target.contains("openbsd") {
        println!("cargo:rustc-link-lib=pthread");
    } else if target.contains("solaris") {
        println!("cargo:rustc-link-lib=socket");
        println!("cargo:rustc-link-lib=posix4");
        println!("cargo:rustc-link-lib=pthread");
    } else if target.contains("apple-darwin") {
        println!("cargo:rustc-link-lib=System");

        // res_init and friends require -lresolv on macOS/iOS.
        // See #41582 and http://blog.achernya.com/2013/03/os-x-has-silly-libsystem.html
        println!("cargo:rustc-link-lib=resolv");
    } else if target.contains("apple-ios") {
        println!("cargo:rustc-link-lib=System");
        println!("cargo:rustc-link-lib=objc");
        println!("cargo:rustc-link-lib=framework=Security");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=resolv");
    } else if target.contains("windows") {
        println!("cargo:rustc-link-lib=advapi32");
        println!("cargo:rustc-link-lib=ws2_32");
        println!("cargo:rustc-link-lib=userenv");
        println!("cargo:rustc-link-lib=shell32");
    } else if target.contains("fuchsia") {
        // use system-provided libbacktrace
        if cfg!(feature = "backtrace") {
            println!("cargo:rustc-link-lib=backtrace");
        }
        println!("cargo:rustc-link-lib=magenta");
        println!("cargo:rustc-link-lib=mxio");
        println!("cargo:rustc-link-lib=launchpad"); // for std::process
    }
}
