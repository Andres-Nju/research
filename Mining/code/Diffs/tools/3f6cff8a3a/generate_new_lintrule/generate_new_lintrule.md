File_Code/tools/3f6cff8a3a/generate_new_lintrule/generate_new_lintrule_after.rs --- Rust
20         r#"use crate::semantic_services::Semantic;                                                                                                        20         r#"use crate::semantic_services::Semantic;
21 use rome_analyze::{{                                                                                                                                      21 use rome_analyze::{{
22     context::RuleContext, declare_rule, Rule, RuleDiagnostic,                                                                                             22     context::RuleContext, declare_rule, Rule, RuleDiagnostic,
23 }};                                                                                                                                                       23 }};
24 use rome_console::markup;                                                                                                                                 24 use rome_console::markup;
25 use rome_js_semantic::{{Reference, ReferencesExtensions}};                                                                                                25 use rome_js_semantic::{{Reference, ReferencesExtensions}};
26 use rome_js_syntax::JsIdentifierBinding;                                                                                                                  26 use rome_js_syntax::JsIdentifierBinding;
27                                                                                                                                                           27 
28 declare_rule! {{                                                                                                                                          28 declare_rule! {{
29     /// Succinct description of the rule.                                                                                                                 29     /// Succinct description of the rule.
30     ///                                                                                                                                                   30     ///
31     /// Put context and details about the rule.                                                                                                           31     /// Put context and details about the rule.
32     /// As a starting point, you can take the description of the corresponding _ESLint_ rule (if any).                                                    32     /// As a starting point, you can take the description of the corresponding _ESLint_ rule (if any).
33     ///                                                                                                                                                   33     ///
34     /// Try to stay consistent with the descriptions of implemented rules.                                                                                34     /// Try to stay consistent with the descriptions of implemented rules.
35     ///                                                                                                                                                   35     ///
36     /// Add a link to the corresponding ESLint rule (if any):                                                                                             36     /// Add a link to the corresponding ESLint rule (if any):
37     ///                                                                                                                                                   37     ///
38     /// Source: https://eslint.org/docs/latest/rules/<rule-name>                                                                                          38     /// Source: https://eslint.org/docs/latest/rules/rule-name
39     ///                                                                                                                                                   39     ///
40     /// ## Examples                                                                                                                                       40     /// ## Examples
41     ///                                                                                                                                                   41     ///
42     /// ### Invalid                                                                                                                                       42     /// ### Invalid
43     ///                                                                                                                                                   43     ///
44     /// ```js,expect_diagnostic                                                                                                                           44     /// ```js,expect_diagnostic
45     /// var a = 1;                                                                                                                                        45     /// var a = 1;
46     /// a = 2;                                                                                                                                            46     /// a = 2;
47     /// ```                                                                                                                                               47     /// ```
48     ///                                                                                                                                                   48     ///
49     /// ## Valid                                                                                                                                          49     /// ## Valid
50     ///                                                                                                                                                   50     ///
51     /// ```js                                                                                                                                             51     /// ```js
52     /// var a = 1;                                                                                                                                        52     /// var a = 1;
53     /// ```                                                                                                                                               53     /// ```
54     ///                                                                                                                                                   54     ///
55     pub(crate) {rule_name_upper_camel} {{                                                                                                                 55     pub(crate) {rule_name_upper_camel} {{
56         version: "next",                                                                                                                                  56         version: "next",
57         name: "{rule_name_lower_camel}",                                                                                                                  57         name: "{rule_name_lower_camel}",
58         recommended: false,                                                                                                                               58         recommended: false,
59     }}                                                                                                                                                    59     }}
60 }}                                                                                                                                                        60 }}
61                                                                                                                                                           61 
62 impl Rule for {rule_name_upper_camel} {{                                                                                                                  62 impl Rule for {rule_name_upper_camel} {{
63     type Query = Semantic<JsIdentifierBinding>;                                                                                                           63     type Query = Semantic<JsIdentifierBinding>;
64     type State = Reference;                                                                                                                               64     type State = Reference;
65     type Signals = Vec<Self::State>;                                                                                                                      65     type Signals = Vec<Self::State>;
66     type Options = ();                                                                                                                                    66     type Options = ();
67                                                                                                                                                           67 
68     fn run(ctx: &RuleContext<Self>) -> Self::Signals {{                                                                                                   68     fn run(ctx: &RuleContext<Self>) -> Self::Signals {{
69         let binding = ctx.query();                                                                                                                        69         let binding = ctx.query();
70         let model = ctx.model();                                                                                                                          70         let model = ctx.model();
71                                                                                                                                                           71 
72         binding.all_references(model).collect()                                                                                                           72         binding.all_references(model).collect()
73     }}                                                                                                                                                    73     }}
74                                                                                                                                                           74 
75     fn diagnostic(_: &RuleContext<Self>, reference: &Self::State) -> Option<RuleDiagnostic> {{                                                            75     fn diagnostic(_: &RuleContext<Self>, reference: &Self::State) -> Option<RuleDiagnostic> {{
76         Some(                                                                                                                                             76         Some(
77             RuleDiagnostic::new(                                                                                                                          77             RuleDiagnostic::new(
78                 rule_category!(),                                                                                                                         78                 rule_category!(),
79                 reference.syntax().text_trimmed_range(),                                                                                                  79                 reference.syntax().text_trimmed_range(),
80                 markup! {{                                                                                                                                80                 markup! {{
81                     "Variable is read here."                                                                                                              81                     "Variable is read here."
82                 }},                                                                                                                                       82                 }},
83             )                                                                                                                                             83             )
84             .note(markup! {{                                                                                                                              84             .note(markup! {{
85                 "This note will give you more information."                                                                                               85                 "This note will give you more information."
86             }}),                                                                                                                                          86             }}),
87         )                                                                                                                                                 87         )
88     }}                                                                                                                                                    88     }}
89 }}                                                                                                                                                        89 }}
90 "#                                                                                                                                                        90 "#

