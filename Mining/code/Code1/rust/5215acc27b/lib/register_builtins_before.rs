pub fn register_builtins(store: &mut lint::LintStore, sess: Option<&Session>) {
    macro_rules! add_builtin {
        ($sess:ident, $($name:ident),*,) => (
            {$(
                store.register_late_pass($sess, false, box $name);
                )*}
            )
    }

    macro_rules! add_early_builtin {
        ($sess:ident, $($name:ident),*,) => (
            {$(
                store.register_early_pass($sess, false, box $name);
                )*}
            )
    }

    macro_rules! add_builtin_with_new {
        ($sess:ident, $($name:ident),*,) => (
            {$(
                store.register_late_pass($sess, false, box $name::new());
                )*}
            )
    }

    macro_rules! add_lint_group {
        ($sess:ident, $name:expr, $($lint:ident),*) => (
            store.register_group($sess, false, $name, vec![$(LintId::of($lint)),*]);
            )
    }

    add_early_builtin!(sess,
                       UnusedParens,
                       );

    add_builtin!(sess,
                 HardwiredLints,
                 WhileTrue,
                 ImproperCTypes,
                 BoxPointers,
                 UnusedAttributes,
                 PathStatements,
                 UnusedResults,
                 NonCamelCaseTypes,
                 NonSnakeCase,
                 NonUpperCaseGlobals,
                 UnusedImportBraces,
                 NonShorthandFieldPatterns,
                 UnusedUnsafe,
                 UnsafeCode,
                 UnusedMut,
                 UnusedAllocation,
                 MissingCopyImplementations,
                 UnstableFeatures,
                 Deprecated,
                 UnconditionalRecursion,
                 InvalidNoMangleItems,
                 PluginAsLibrary,
                 DropWithReprExtern,
                 MutableTransmutes,
                 );

    add_builtin_with_new!(sess,
                          TypeLimits,
                          MissingDoc,
                          MissingDebugImplementations,
                          );

    add_lint_group!(sess, "bad_style",
                    NON_CAMEL_CASE_TYPES, NON_SNAKE_CASE, NON_UPPER_CASE_GLOBALS);

    add_lint_group!(sess, "unused",
                    UNUSED_IMPORTS, UNUSED_VARIABLES, UNUSED_ASSIGNMENTS, DEAD_CODE,
                    UNUSED_MUT, UNREACHABLE_CODE, UNUSED_MUST_USE,
                    UNUSED_UNSAFE, PATH_STATEMENTS, UNUSED_ATTRIBUTES);

    // Guidelines for creating a future incompatibility lint:
    //
    // - Create a lint defaulting to warn as normal, with ideally the same error
    //   message you would normally give
    // - Add a suitable reference, typically an RFC or tracking issue. Go ahead
    //   and include the full URL.
    // - Later, change lint to error
    // - Eventually, remove lint
    store.register_future_incompatible(sess, vec![
        FutureIncompatibleInfo {
            id: LintId::of(PRIVATE_IN_PUBLIC),
            reference: "the explanation for E0446 (`--explain E0446`)",
        },
        FutureIncompatibleInfo {
            id: LintId::of(INACCESSIBLE_EXTERN_CRATE),
            reference: "PR 31362 <https://github.com/rust-lang/rust/pull/31362>",
        },
        FutureIncompatibleInfo {
            id: LintId::of(INVALID_TYPE_PARAM_DEFAULT),
            reference: "PR 30742 <https://github.com/rust-lang/rust/pull/30724>",
        },
        FutureIncompatibleInfo {
            id: LintId::of(SUPER_OR_SELF_IN_GLOBAL_PATH),
            reference: "PR #32403 <https://github.com/rust-lang/rust/pull/32403>",
        },
        FutureIncompatibleInfo {
            id: LintId::of(MATCH_OF_UNIT_VARIANT_VIA_PAREN_DOTDOT),
            reference: "RFC 218 <https://github.com/rust-lang/rfcs/blob/\
                        master/text/0218-empty-struct-with-braces.md>",
        },
        FutureIncompatibleInfo {
            id: LintId::of(TRANSMUTE_FROM_FN_ITEM_TYPES),
            reference: "issue #19925 <https://github.com/rust-lang/rust/issues/19925>",
        },
        FutureIncompatibleInfo {
            id: LintId::of(OVERLAPPING_INHERENT_IMPLS),
            reference: "issue #22889 <https://github.com/rust-lang/rust/issues/22889>",
        },
        FutureIncompatibleInfo {
            id: LintId::of(ILLEGAL_FLOATING_POINT_CONSTANT_PATTERN),
            reference: "RFC 1445 <https://github.com/rust-lang/rfcs/pull/1445>",
        },
        FutureIncompatibleInfo {
            id: LintId::of(ILLEGAL_STRUCT_OR_ENUM_CONSTANT_PATTERN),
            reference: "RFC 1445 <https://github.com/rust-lang/rfcs/pull/1445>",
        },
        FutureIncompatibleInfo {
            id: LintId::of(UNSIZED_IN_TUPLE),
            reference: "issue #33242 <https://github.com/rust-lang/rust/issues/33242>",
        },
        FutureIncompatibleInfo {
            id: LintId::of(OBJECT_UNSAFE_FRAGMENT),
            reference: "issue #33243 <https://github.com/rust-lang/rust/issues/33243>",
        },
        FutureIncompatibleInfo {
            id: LintId::of(HR_LIFETIME_IN_ASSOC_TYPE),
            reference: "issue #33685 <https://github.com/rust-lang/rust/issues/33685>",
        },
        FutureIncompatibleInfo {
            id: LintId::of(LIFETIME_UNDERSCORE),
            reference: "RFC 1177 <https://github.com/rust-lang/rfcs/pull/1177>",
        },
        ]);

    // We have one lint pass defined specially
    store.register_late_pass(sess, false, box lint::GatherNodeLevels);

    // Register renamed and removed lints
    store.register_renamed("unknown_features", "unused_features");
    store.register_removed("unsigned_negation", "replaced by negate_unsigned feature gate");
    store.register_removed("negate_unsigned", "cast a signed value instead");
    store.register_removed("raw_pointer_derive", "using derive with raw pointers is ok");
    // This was renamed to raw_pointer_derive, which was then removed,
    // so it is also considered removed
    store.register_removed("raw_pointer_deriving", "using derive with raw pointers is ok");
}
