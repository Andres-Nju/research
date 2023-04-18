fn main() {
    let (args, passthrough) = parse_arguments();

    // Find all the native shared libraries that exist in the target directory.
    let native_shared_libs = find_native_libs(&args);

    // Get the SDK path from the ANDROID_HOME env.
    let sdk_path = env::var("ANDROID_HOME").ok().expect("Please set the ANDROID_HOME environment variable");
    let sdk_path = Path::new(&sdk_path);

    // Get the NDK path from NDK_HOME env.
    let ndk_path = env::var("NDK_HOME").ok().expect("Please set the NDK_HOME environment variable");
    let ndk_path = Path::new(&ndk_path);

    // Get the target android platform from ANDROID_PLATFORM env. Expecting "android-{version}"
    let android_platform = env::var("ANDROID_PLATFORM")
        .ok()
        .expect("Please set the ANDROID_PLATFORM environment variable")

    // Get the standalone NDK path from NDK_STANDALONE env.
    //  let standalone_path = env::var("NDK_STANDALONE").ok().unwrap_or("/opt/ndk_standalone".to_string());
    //  let standalone_path = Path::new(&standalone_path);

    let debug = passthrough.contains(&"-d".to_string());

    // Set the build directory that will contain all the necessary files to create the apk
    let directory = args.root_path.join("support").join("android").join("apk");
    let resdir = args.root_path.join("resources/");

    // executing ndk-build
    env::set_var("V", "1");
    if debug {
        env::set_var("NDK_DEBUG", "1");
        env::set_var("APP_OPTIM", "0");
    } else {
        // Overrides android:debuggable propery in the .xml file.
        env::set_var("APP_OPTIM", "1");
    }

    // Copy libservo.so into the jni folder for inclusion in the build
    // TODO: pass/detect target architecture
    {
        let source = &args.target_path.join("libservo.so");
        let target_dir = &directory.join("jni").join("armeabi");
        let _ = DirBuilder::new().recursive(true).create(target_dir);
        let target = target_dir.join("libmain.so");
        println!("Copying the file {:?} to {:?}", source, target);
        fs::copy(source, target).unwrap();
    }

    let ndkcmd = Command::new(ndk_path.join("ndk-build"))
                              .arg("-B")
                              .stdout(Stdio::inherit())
                              .stderr(Stdio::inherit())
                              .current_dir(directory.clone())
                              .status();
    if ndkcmd.is_err() || ndkcmd.unwrap().code().unwrap() != 0 {
        println!("Error while executing program `ndk-build`, or missing program.");
        process::exit(1);
    }

    // Copy the additional native libs into the libs directory.
    for (name, path) in native_shared_libs.iter() {
        let target = &directory.join("libs").join("armeabi").join(name);
        println!("Copying the file {:?} to {:?}", name, target);
        fs::copy(path, target).unwrap();
    }

    // Copy over the resources
    let cpcmd = Command::new("cp")
                             .arg("-R")
                             .arg(&resdir)
                             .arg(&directory.join("assets"))
                             .stdout(Stdio::inherit())
                             .stderr(Stdio::inherit())
                             .current_dir(directory.clone())
                             .status();
    if cpcmd.is_err() || cpcmd.unwrap().code().unwrap() != 0 {
        println!("Error while copying files from the resources dir to the assets dir");
        process::exit(1);
    }

    // Update the project
    let androidcmd = Command::new(sdk_path.join("tools").join("android"))
                                  .arg("update")
                                  .arg("project")
                                  .arg("--name")
                                  .arg("Servo")
                                  .arg("--target")
                                  .arg(&android_platform)
                                  .arg("--path")
                                  .arg(".")
                                  .stdout(Stdio::inherit())
                                  .stderr(Stdio::inherit())
                                  .current_dir(directory.clone())
                                  .status();
    if androidcmd.is_err() || androidcmd.unwrap().code().unwrap() != 0 {
        println!("Error while updating the project with the android command");
        process::exit(1);
    }

    // Build the APK
    let mut antcmd = Command::new("ant");
    if debug {
        antcmd.arg("debug");
    } else {
        antcmd.arg("release");
    }
    let antresult = antcmd
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir(directory.clone())
        .status();
    if antresult.is_err() || antresult.unwrap().code().unwrap() != 0 {
        println!("Error while executing program `ant`, or missing program.");
        process::exit(1);
    }

    // Copying apk file to the requested output
    // Release builds also need to be signed. For now, we use a simple debug
    // signing key.
    if debug {
        fs::copy(&directory.join("bin").join("Servo-debug.apk"),
                 &args.output).unwrap();
    } else {
        let keystore_dir = env::home_dir().expect("Please have a home directory");
        let keystore_dir = Path::new(&keystore_dir).join(".keystore");
        let keytoolcmd = Command::new("keytool")
                                  .arg("-list")
                                  .arg("-storepass")
                                  .arg("android")
                                  .arg("-alias")
                                  .arg("androiddebugkey")
                                  .arg("-keystore")
                                  .arg(&keystore_dir)
                                  .stdout(Stdio::inherit())
                                  .stderr(Stdio::inherit())
                                  .current_dir(directory.clone())
                                  .status();
        if keytoolcmd.is_err() || keytoolcmd.unwrap().code().unwrap() != 0 {
            let keytoolcreatecmd = Command::new("keytool")
                                  .arg("-genkeypair")
                                  .arg("-keystore")
                                  .arg(&keystore_dir)
                                  .arg("-storepass")
                                  .arg("android")
                                  .arg("-alias")
                                  .arg("androiddebugkey")
                                  .arg("-keypass")
                                  .arg("android")
                                  .arg("-dname")
                                  .arg("CN=Android Debug,O=Android,C=US")
                                  .arg("-keyalg")
                                  .arg("RSA")
                                  .arg("-validity")
                                  .arg("365")
                                  .stdout(Stdio::inherit())
                                  .stderr(Stdio::inherit())
                                  .current_dir(directory.clone())
                                  .status();
            if keytoolcreatecmd.is_err() ||
               keytoolcreatecmd.unwrap().code().unwrap() != 0 {
                   println!("Error while using `keytool` to create the debug keystore.");
                   process::exit(1);
               }
        }

        let jarsigncmd = Command::new("jarsigner")
                                  .arg("-digestalg")
                                  .arg("SHA1")
                                  .arg("-sigalg")
                                  .arg("MD5withRSA")
                                  .arg("-storepass")
                                  .arg("android")
                                  .arg("-keystore")
                                  .arg(&keystore_dir)
                                  .arg(&directory.join("bin").join("Servo-release-unsigned.apk"))
                                  .arg("androiddebugkey")
                                  .stdout(Stdio::inherit())
                                  .stderr(Stdio::inherit())
                                  .current_dir(directory.clone())
                                  .status();
        if jarsigncmd.is_err() || jarsigncmd.unwrap().code().unwrap() != 0 {
            println!("Error while using `jarsign` to sign the APK.");
            process::exit(1);
        }

        fs::copy(&directory.join("bin").join("Servo-release-unsigned.apk"),
                 &args.output).unwrap();
    }

}
