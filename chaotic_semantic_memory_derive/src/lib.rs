use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(Concept)]
pub fn derive_concept(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let Data::Struct(data) = input.data else {
        return quote! { compile_error!("Concept derive only supports structs"); }.into();
    };

    let has_vector = match data.fields {
        Fields::Named(ref fields) => fields.named.iter().any(|f| {
            f.ident
                .as_ref()
                .map(|id| id == "vector")
                .unwrap_or(false)
        }),
        _ => false,
    };

    let vector_expr = if has_vector {
        quote! { self.vector }
    } else {
        quote! { ::chaotic_semantic_memory::HVec10240::random() }
    };

    let expanded = quote! {
        impl #name {
            pub fn into_concept(self, id: impl Into<String>) -> ::chaotic_semantic_memory::Result<::chaotic_semantic_memory::Concept> {
                let concept = ::chaotic_semantic_memory::ConceptBuilder::new(id)
                    .with_vector(#vector_expr)
                    .build()?;
                Ok(concept)
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(HypervectorField)]
pub fn derive_hypervector_field(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let expanded = quote! {
        impl #name {
            pub fn to_hypervector(&self) -> ::chaotic_semantic_memory::HVec10240 {
                ::chaotic_semantic_memory::HVec10240::random()
            }
        }
    };

    TokenStream::from(expanded)
}
