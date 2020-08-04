#![recursion_limit = "128"]

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

#[proc_macro_attribute]
pub fn lapack(_attr: TokenStream, _func: TokenStream) -> TokenStream {
    let ts = quote::quote! {};
    ts.into()
}

fn convert(func: TokenStream2) -> TokenStream2 {
    let f: syn::ForeignItemFn = syn::parse2(func).unwrap();
    for input in &f.sig.inputs {
        match input {
            syn::FnArg::Typed(pat) => {
                dbg!(&pat.pat);
                dbg!(&pat.ty);
            }
            _ => panic!(),
        }
    }
    quote::quote! { #f }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dgetrs_convert() {
        let dgetrs = r#"
        pub fn dgetrs_(
            trans: *const c_char,
            n: *const c_int,
            nrhs: *const c_int,
            A: *const f64,
            lda: *const c_int,
            ipiv: *const c_int,
            B: *mut f64,
            ldb: *const c_int,
            info: *mut c_int,
        );
        "#;
        let wrapped = convert(syn::parse_str(dgetrs).unwrap());
        let expected = r#"
        pub unsafe fn dgetrs(
            trans: u8,
            n: i32,
            nrhs: i32,
            a: &[f64],
            lda: i32,
            ipiv: &[i32],
            b: &mut [f64],
            ldb: i32,
            info: &mut i32
        ) {
            dgetrs_(
                &(trans as c_char),
                &n,
                &nrhs,
                a.as_ptr(),
                &lda,
                ipiv.as_ptr(),
                b.as_mut_ptr(),
                &ldb,
                info,
            )
        }
        "#;
        let expected: TokenStream2 = syn::parse_str(expected).unwrap();
        assert_eq!(wrapped.to_string(), expected.to_string());
    }
}
