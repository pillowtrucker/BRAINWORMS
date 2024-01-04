use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;

mod derive_scenic_partial;
use crate::derive_scenic_partial::analyze;
use crate::derive_scenic_partial::codegen;
use crate::derive_scenic_partial::lower;
use crate::derive_scenic_partial::parse;

#[proc_macro_derive(Scenic, attributes(define))]
#[proc_macro_error]
pub fn derive_scenic_partial(ts: TokenStream) -> TokenStream {
    let ast = parse::parse(ts.clone().into());
    let model = analyze::analyze(ast);
    let ir = lower::lower(model);
    let _ = codegen::codegen(ir);
    TokenStream::new()
}
