use proc_macro::TokenStream;

use proc_macro2::Ident;
use quote::quote;
use syn::DeriveInput;

#[proc_macro_derive(Event)]
/// Implements [`Debug`] for a struct or enum, with certain fields redacted.
///
/// See the [crate level documentation](index.html) for flags and modifiers.
pub fn derive_event(item: TokenStream) -> TokenStream {
    let derive_input: DeriveInput = syn::parse_macro_input!(item as DeriveInput);

    let ident: Ident = derive_input.ident;

    quote!(
        impl esrs::event::Event for #ident {}
    )
    .into()
}
