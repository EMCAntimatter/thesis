extern crate proc_macro;
use proc_macro::TokenStream;

use proc_macro2::Span;
use quote::quote;

/// Example of user-defined [derive mode macro][1]
///
/// [1]: https://doc.rust-lang.org/reference/procedural-macros.html#derive-mode-macros
#[proc_macro_derive(TailQIterator)]
pub fn tailq_iterator(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;
    let field = match ast.data {
        syn::Data::Struct(s) => {
            let field = s.fields.iter().next().unwrap();
            let field_name = field.ident.as_ref().unwrap();
            if field_name.to_string().contains("inner") {
                let iter_name_str = name.to_string() + "TailQIterator";

                let iter_name = syn::Ident::new(iter_name_str.as_str(), Span::call_site());

                let field_name = field.ident.as_ref().unwrap();

                let tokens = quote! {
                    struct #iter_name {
                        current: Option<#name>
                    }

                    impl Iterator for #iter_name {
                        type Item = #name;

                        fn next(&mut self) -> Option<Self::Item> {
                            if let Some(cur) = self.current.take() {
                                let next = unsafe { cur.#field_name.read().next.tqe_next };
                                if next.is_null() {
                                    self.current = None;
                                } else {
                                    self.current = Some(#name {
                                        inner: next
                                    });
                                }
                                Some(cur)
                            } else {
                                None
                            }
                        }
                    }
                };
                tokens
            } else {
                unreachable!()
            }
        }
        syn::Data::Enum(_) | syn::Data::Union(_) => unimplemented!(),
    };
    field.into()
}
