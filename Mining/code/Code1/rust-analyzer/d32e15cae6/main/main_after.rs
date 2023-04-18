fn main() -> Result<()> {
    // re-enable and verify on windows after #1400
    // std::env::set_var("RUST_BACKTRACE", "short");
    let logger = Logger::with_env_or_str("error").duplicate_to_stderr(Duplicate::All);
    match std::env::var("RA_LOG_DIR") {
        Ok(ref v) if v == "1" => logger.log_to_file().directory("log").start()?,
        _ => logger.start()?,
    };
    ra_prof::set_filter(match std::env::var("RA_PROFILE") {
        Ok(spec) => ra_prof::Filter::from_spec(&spec),
        Err(_) => ra_prof::Filter::disabled(),
    });
    log::info!("lifecycle: server started");
    match ::std::panic::catch_unwind(main_inner) {
        Ok(res) => {
            log::info!("lifecycle: terminating process with {:?}", res);
            res
        }
        Err(_) => {
            log::error!("server panicked");
            failure::bail!("server panicked")
        }
    }
}
