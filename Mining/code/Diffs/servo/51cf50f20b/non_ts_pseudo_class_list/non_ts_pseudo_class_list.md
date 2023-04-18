File_Code/servo/51cf50f20b/non_ts_pseudo_class_list/non_ts_pseudo_class_list_after.rs --- Rust
 5 /*                                                                                                                                                         5 /*
 6  * This file contains a helper macro includes all supported non-tree-structural                                                                            6  * This file contains a helper macro includes all supported non-tree-structural
 7  * pseudo-classes.                                                                                                                                         7  * pseudo-classes.
 8  *                                                                                                                                                         8  *
 9                                                                                                                                                            9 
10  * FIXME: Find a way to autogenerate this file.                                                                                                           10  * FIXME: Find a way to autogenerate this file.
11  *                                                                                                                                                        11  *
12  * Expected usage is as follows:                                                                                                                          12  * Expected usage is as follows:
13  * ```                                                                                                                                                    13  * ```
14  * macro_rules! pseudo_class_macro{                                                                                                                       14  * macro_rules! pseudo_class_macro{
15  *     (bare: [$(($css:expr, $name:ident, $gecko_type:tt, $state:tt, $flags:tt),)*],                                                                      15  *     (bare: [$(($css:expr, $name:ident, $gecko_type:tt, $state:tt, $flags:tt),)*],
16  *      string: [$(($s_css:expr, $s_name:ident, $s_gecko_type:tt, $s_state:tt, $s_flags:tt),)*]) => {                                                     16  *      string: [$(($s_css:expr, $s_name:ident, $s_gecko_type:tt, $s_state:tt, $s_flags:tt),)*]) => {
17  *         // do stuff                                                                                                                                    17  *         // do stuff
18  *     }                                                                                                                                                  18  *     }
19  * }                                                                                                                                                      19  * }
20  * apply_non_ts_list!(pseudo_class_macro)                                                                                                                 20  * apply_non_ts_list!(pseudo_class_macro)
21  * ```                                                                                                                                                    21  * ```
22  *                                                                                                                                                        22  *
23  * The string variables will be applied to pseudoclasses that are of the form                                                                             23  * The string variables will be applied to pseudoclasses that are of the form
24  * of a function with a string argument.                                                                                                                  24  * of a function with a string argument.
25  *                                                                                                                                                        25  *
26  * Pending pseudo-classes:                                                                                                                                26  * Pending pseudo-classes:
27  *                                                                                                                                                        27  *
28  *  :-moz-is-html -> Used only in UA sheets, should be easy to support.                                                                                   28  *  :-moz-is-html -> Used only in UA sheets, should be easy to support.
29  *  :-moz-native-anonymous -> For devtools, seems easy-ish?                                                                                               29  *  :-moz-native-anonymous -> For devtools, seems easy-ish?
30  *  :-moz-bound-element -> Seems unused, should be easy to remove.                                                                                        30  *  :-moz-bound-element -> Seems unused, should be easy to remove.
31  *                                                                                                                                                        31  *
32  *  :-moz-lwtheme, :-moz-lwtheme-brighttext, :-moz-lwtheme-darktext,                                                                                      32  *  :-moz-lwtheme, :-moz-lwtheme-brighttext, :-moz-lwtheme-darktext,
33  *  :-moz-window-inactive.                                                                                                                                33  *  :-moz-window-inactive.
34  *                                                                                                                                                        34  *
35  *  :scope -> <style scoped>, pending discussion.                                                                                                         35  *  :scope -> <style scoped>, pending discussion.
36  *                                                                                                                                                        36  *
37  * This follows the order defined in layout/style/nsCSSPseudoClassList.h when                                                                             37  * This follows the order defined in layout/style/nsCSSPseudoClassList.h when
38  * possible.                                                                                                                                              38  * possible.
39  *                                                                                                                                                        39  *
40  * $gecko_type can be either "_" or an ident in Gecko's CSSPseudoClassType.                                                                               40  * $gecko_type can be either "_" or an ident in Gecko's CSSPseudoClassType.
41  * $state can be either "_" or an expression of type ElementState.                                                                                        41  * $state can be either "_" or an expression of type ElementState.  If present,
..                                                                                                                                                           42  *        the semantics are that the pseudo-class matches if any of the bits in
..                                                                                                                                                           43  *        $state are set on the element.
42  * $flags can be either "_" or an expression of type NonTSPseudoClassFlag,                                                                                44  * $flags can be either "_" or an expression of type NonTSPseudoClassFlag,
43  * see selector_parser.rs for more details.                                                                                                               45  * see selector_parser.rs for more details.
44  */                                                                                                                                                       46  */

