  fn resolve_package_from_deno_module(
    &self,
    pkg_req: &NpmPackageReq,
  ) -> Result<LocalNpmPackageInfo, AnyError>;

  /// Resolves an npm package from an npm package referrer.
  fn resolve_package_from_package(
    &self,
    name: &str,
    referrer: &ModuleSpecifier,
  ) -> Result<LocalNpmPackageInfo, AnyError>;

  /// Resolve the root folder of the package the provided specifier is in.
  ///
  /// This will error when the provided specifier is not in an npm package.
  fn resolve_package_from_specifier(
    &self,
    specifier: &ModuleSpecifier,
  ) -> Result<LocalNpmPackageInfo, AnyError>;

  /// Gets if the provided specifier is in an npm package.
  fn in_npm_package(&self, specifier: &ModuleSpecifier) -> bool {
    self.resolve_package_from_specifier(specifier).is_ok()
  }
