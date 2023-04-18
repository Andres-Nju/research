fn test_permissions_net_listen_allow_localhost() {
  let (_, err, code) = util::run_and_collect_output(
			"run --allow-net=localhost complex_permissions_test.ts netListen localhost:4545 localhost:4546 localhost:4547",
			None,
			None,
			false,
		);
  assert_eq!(code, 0);
  assert!(!err.contains(util::PERMISSION_DENIED_PATTERN));
}
