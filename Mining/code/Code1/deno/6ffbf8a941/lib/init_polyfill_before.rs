pub fn init_polyfill() -> Extension {
  let esm_files = include_js_files!(
    dir "polyfills",
    "_core.ts",
    "_events.mjs",
    "_fs/_fs_access.ts",
    "_fs/_fs_appendFile.ts",
    "_fs/_fs_chmod.ts",
    "_fs/_fs_chown.ts",
    "_fs/_fs_close.ts",
    "_fs/_fs_common.ts",
    "_fs/_fs_constants.ts",
    "_fs/_fs_copy.ts",
    "_fs/_fs_dir.ts",
    "_fs/_fs_dirent.ts",
    "_fs/_fs_exists.ts",
    "_fs/_fs_fdatasync.ts",
    "_fs/_fs_fstat.ts",
    "_fs/_fs_fsync.ts",
    "_fs/_fs_ftruncate.ts",
    "_fs/_fs_futimes.ts",
    "_fs/_fs_link.ts",
    "_fs/_fs_lstat.ts",
    "_fs/_fs_mkdir.ts",
    "_fs/_fs_mkdtemp.ts",
    "_fs/_fs_open.ts",
    "_fs/_fs_opendir.ts",
    "_fs/_fs_read.ts",
    "_fs/_fs_readdir.ts",
    "_fs/_fs_readFile.ts",
    "_fs/_fs_readlink.ts",
    "_fs/_fs_realpath.ts",
    "_fs/_fs_rename.ts",
    "_fs/_fs_rm.ts",
    "_fs/_fs_rmdir.ts",
    "_fs/_fs_stat.ts",
    "_fs/_fs_symlink.ts",
    "_fs/_fs_truncate.ts",
    "_fs/_fs_unlink.ts",
    "_fs/_fs_utimes.ts",
    "_fs/_fs_watch.ts",
    "_fs/_fs_write.mjs",
    "_fs/_fs_writeFile.ts",
    "_fs/_fs_writev.mjs",
    "_http_agent.mjs",
    "_http_common.ts",
    "_http_outgoing.ts",
    "_next_tick.ts",
    "_pako.mjs",
    "_process/exiting.ts",
    "_process/process.ts",
    "_process/stdio.mjs",
    "_process/streams.mjs",
    "_readline.mjs",
    "_stream.mjs",
    "_tls_common.ts",
    "_tls_wrap.ts",
    "_util/_util_callbackify.ts",
    "_util/asserts.ts",
    "_util/async.ts",
    "_util/os.ts",
    "_util/std_asserts.ts",
    "_util/std_fmt_colors.ts",
    "_util/std_testing_diff.ts",
    "_utils.ts",
    "_zlib_binding.mjs",
    "_zlib.mjs",
    "assert.ts",
    "assert/strict.ts",
    "assertion_error.ts",
    "async_hooks.ts",
    "buffer.ts",
    "child_process.ts",
    "cluster.ts",
    "console.ts",
    "constants.ts",
    "crypto.ts",
    "dgram.ts",
    "diagnostics_channel.ts",
    "dns.ts",
    "dns/promises.ts",
    "domain.ts",
    "events.ts",
    "fs.ts",
    "fs/promises.ts",
    "global.ts",
    "http.ts",
    "http2.ts",
    "https.ts",
    "inspector.ts",
    "internal_binding/_libuv_winerror.ts",
    "internal_binding/_listen.ts",
    "internal_binding/_node.ts",
    "internal_binding/_timingSafeEqual.ts",
    "internal_binding/_utils.ts",
    "internal_binding/ares.ts",
    "internal_binding/async_wrap.ts",
    "internal_binding/buffer.ts",
    "internal_binding/cares_wrap.ts",
    "internal_binding/config.ts",
    "internal_binding/connection_wrap.ts",
    "internal_binding/constants.ts",
    "internal_binding/contextify.ts",
    "internal_binding/credentials.ts",
    "internal_binding/crypto.ts",
    "internal_binding/errors.ts",
    "internal_binding/fs_dir.ts",
    "internal_binding/fs_event_wrap.ts",
    "internal_binding/fs.ts",
    "internal_binding/handle_wrap.ts",
    "internal_binding/heap_utils.ts",
    "internal_binding/http_parser.ts",
    "internal_binding/icu.ts",
    "internal_binding/inspector.ts",
    "internal_binding/js_stream.ts",
    "internal_binding/messaging.ts",
    "internal_binding/mod.ts",
    "internal_binding/module_wrap.ts",
    "internal_binding/native_module.ts",
    "internal_binding/natives.ts",
    "internal_binding/node_file.ts",
    "internal_binding/node_options.ts",
    "internal_binding/options.ts",
    "internal_binding/os.ts",
    "internal_binding/performance.ts",
    "internal_binding/pipe_wrap.ts",
    "internal_binding/process_methods.ts",
    "internal_binding/report.ts",
    "internal_binding/serdes.ts",
    "internal_binding/signal_wrap.ts",
    "internal_binding/spawn_sync.ts",
    "internal_binding/stream_wrap.ts",
    "internal_binding/string_decoder.ts",
    "internal_binding/symbols.ts",
    "internal_binding/task_queue.ts",
    "internal_binding/tcp_wrap.ts",
    "internal_binding/timers.ts",
    "internal_binding/tls_wrap.ts",
    "internal_binding/trace_events.ts",
    "internal_binding/tty_wrap.ts",
    "internal_binding/types.ts",
    "internal_binding/udp_wrap.ts",
    "internal_binding/url.ts",
    "internal_binding/util.ts",
    "internal_binding/uv.ts",
    "internal_binding/v8.ts",
    "internal_binding/worker.ts",
    "internal_binding/zlib.ts",
    "internal/assert.mjs",
    "internal/async_hooks.ts",
    "internal/blob.mjs",
    "internal/buffer.mjs",
    "internal/child_process.ts",
    "internal/cli_table.ts",
    "internal/console/constructor.mjs",
    "internal/constants.ts",
    "internal/crypto/_hex.ts",
    "internal/crypto/_keys.ts",
    "internal/crypto/_randomBytes.ts",
    "internal/crypto/_randomFill.ts",
    "internal/crypto/_randomInt.ts",
    "internal/crypto/certificate.ts",
    "internal/crypto/cipher.ts",
    "internal/crypto/constants.ts",
    "internal/crypto/diffiehellman.ts",
    "internal/crypto/hash.ts",
    "internal/crypto/hkdf.ts",
    "internal/crypto/keygen.ts",
    "internal/crypto/keys.ts",
    "internal/crypto/pbkdf2.ts",
    "internal/crypto/random.ts",
    "internal/crypto/scrypt.ts",
    "internal/crypto/sig.ts",
    "internal/crypto/types.ts",
    "internal/crypto/util.ts",
    "internal/crypto/x509.ts",
    "internal/dgram.ts",
    "internal/dns/promises.ts",
    "internal/dns/utils.ts",
    "internal/dtrace.ts",
    "internal/error_codes.ts",
    "internal/errors.ts",
    "internal/event_target.mjs",
    "internal/fixed_queue.ts",
    "internal/freelist.ts",
    "internal/fs/streams.mjs",
    "internal/fs/utils.mjs",
    "internal/hide_stack_frames.ts",
    "internal/http.ts",
    "internal/idna.ts",
    "internal/net.ts",
    "internal/normalize_encoding.mjs",
    "internal/options.ts",
    "internal/primordials.mjs",
    "internal/process/per_thread.mjs",
    "internal/querystring.ts",
    "internal/readline/callbacks.mjs",
    "internal/readline/emitKeypressEvents.mjs",
    "internal/readline/interface.mjs",
    "internal/readline/promises.mjs",
    "internal/readline/symbols.mjs",
    "internal/readline/utils.mjs",
    "internal/stream_base_commons.ts",
    "internal/streams/add-abort-signal.mjs",
    "internal/streams/buffer_list.mjs",
    "internal/streams/destroy.mjs",
    "internal/streams/duplex.mjs",
    "internal/streams/end-of-stream.mjs",
    "internal/streams/lazy_transform.mjs",
    "internal/streams/legacy.mjs",
    "internal/streams/passthrough.mjs",
    "internal/streams/readable.mjs",
    "internal/streams/state.mjs",
    "internal/streams/transform.mjs",
    "internal/streams/utils.mjs",
    "internal/streams/writable.mjs",
    "internal/test/binding.ts",
    "internal/timers.mjs",
    "internal/url.ts",
    "internal/util.mjs",
    "internal/util/comparisons.ts",
    "internal/util/debuglog.ts",
    "internal/util/inspect.mjs",
    "internal/util/types.ts",
    "internal/validators.mjs",
    "module_all.ts",
    "module_esm.ts",
    "module.js",
    "net.ts",
    "os.ts",
    "path.ts",
    "path/_constants.ts",
    "path/_interface.ts",
    "path/_util.ts",
    "path/common.ts",
    "path/glob.ts",
    "path/mod.ts",
    "path/posix.ts",
    "path/separator.ts",
    "path/win32.ts",
    "perf_hooks.ts",
    "process.ts",
    "punycode.ts",
    "querystring.ts",
    "readline.ts",
    "readline/promises.ts",
    "repl.ts",
    "stream.ts",
    "stream/consumers.mjs",
    "stream/promises.mjs",
    "stream/web.ts",
    "string_decoder.ts",
    "sys.ts",
    "timers.ts",
    "timers/promises.ts",
    "tls.ts",
    "tty.ts",
    "upstream_modules.ts",
    "url.ts",
    "util.ts",
    "util/types.ts",
    "v8.ts",
    "vm.ts",
    "wasi.ts",
    "worker_threads.ts",
    "zlib.ts",
  );

  Extension::builder(env!("CARGO_PKG_NAME"))
    .esm(esm_files)
    .esm_entry_point("internal:deno_node/polyfills/module_all.ts")
    .ops(vec![
      crypto::op_node_create_hash::decl(),
      crypto::op_node_hash_update::decl(),
      crypto::op_node_hash_digest::decl(),
      crypto::op_node_hash_clone::decl(),
      crypto::op_node_private_encrypt::decl(),
      crypto::op_node_private_decrypt::decl(),
      crypto::op_node_public_encrypt::decl(),
      winerror::op_node_sys_to_uv_error::decl(),
      v8::op_v8_cached_data_version_tag::decl(),
      v8::op_v8_get_heap_statistics::decl(),
      idna::op_node_idna_domain_to_ascii::decl(),
      idna::op_node_idna_domain_to_unicode::decl(),
      idna::op_node_idna_punycode_decode::decl(),
      idna::op_node_idna_punycode_encode::decl(),
      op_node_build_os::decl(),
    ])
    .build()
}