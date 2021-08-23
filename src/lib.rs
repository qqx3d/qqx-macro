#![feature(in_band_lifetimes)]
#![feature(const_panic)]

mod polygon;
mod callback;
mod tls;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn qqx(attribute: TokenStream, input: TokenStream) -> TokenStream {
    let attribute = attribute.to_string();

    if let Some(x) = tls::take(&attribute, "polygon(..)") {
        polygon::polygon(x, input)
    } else if let Some(x) = tls::take(&attribute, "callback(..)") {
        callback::callback(x, input)
    } else {
        panic!("Unknown attribute `{}`", attribute)
    }
}
