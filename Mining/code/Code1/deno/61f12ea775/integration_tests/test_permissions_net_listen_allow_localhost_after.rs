fn test_permissions_net_listen_allow_localhost() {
  // Port 4600 is chosen to not colide with those used by tools/http_server.py
  let (_, err, code) = util::run_and_collect_output(
			"run --allow-net=localhost complex_permissions_test.ts netListen localhost:4600",
			None,
			None,
			false,
		);
  assert_eq!(code, 0);
  assert!(!err.contains(util::PERMISSION_DENIED_PATTERN));
}
