use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(Scan)]
pub fn derive_scan(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident,
        data,
        generics,
        ..
    }: DeriveInput = parse_macro_input!(input);

    let fields = match data {
        Data::Struct(struct_data) => struct_data.fields,
        _ => panic!("`Scan` can only be derived for structs"),
    };

    let gc_collection_statements = match fields {
        Fields::Unit => vec![],
        Fields::Named(named_fields) => named_fields
            .named
            .into_iter()
            .map(|field| {
                let field_name = field.ident;
                quote! {
                    gcs.extend((&self.#field_name as &dyn Scan).collect_gcs());
                }
            })
            .collect(),
        _ => unimplemented!(),
    };

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics Scan for #ident #type_generics #where_clause {
            fn collect_gcs(&self) -> Vec<usize> {
                let mut gcs = Vec::new();

                #(#gc_collection_statements)*

                gcs
            }
        }
    };

    expanded.into()
}
