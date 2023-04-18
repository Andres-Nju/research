    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            builder_name,
            step_trait,
            step_names,
            props,
            wrapper_name,
        } = self;

        let DerivePropsInput {
            vis,
            generics,
            props_name,
            ..
        } = props;

        let build_step = self.build_step();
        let impl_steps = self.impl_steps();
        let set_fields = self.set_fields();

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
        let turbofish_generics = ty_generics.as_turbofish();
        let generic_args = to_arguments(generics, build_step.clone());

        // Each builder step implements the `BuilderStep` trait and `step_generics` is used to
        // enforce that.
        let step_generic_param = Ident::new("YEW_PROPS_BUILDER_STEP", Span::call_site());
        let step_generics =
            with_param_bounds(generics, step_generic_param.clone(), (*step_trait).clone());

        let builder = quote! {
            #(
                #[doc(hidden)]
                #[allow(non_camel_case_types)]
                #vis struct #step_names;
            )*

            #[doc(hidden)]
            #vis trait #step_trait {}

            #(impl #step_trait for #step_names {})*

            #[doc(hidden)]
            #vis struct #builder_name#step_generics
                #where_clause
            {
                wrapped: ::std::boxed::Box<#wrapper_name#ty_generics>,
                _marker: ::std::marker::PhantomData<#step_generic_param>,
            }

            #impl_steps

            impl#impl_generics #builder_name<#generic_args> #where_clause {
                #[doc(hidden)]
                #vis fn build(self) -> #props_name#ty_generics {
                    #props_name#turbofish_generics {
                        #(#set_fields)*
                    }
                }
