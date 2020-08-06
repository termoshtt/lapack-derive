#![recursion_limit = "128"]

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

type Args = syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>;
type Call = syn::punctuated::Punctuated<syn::Expr, syn::token::Comma>;

#[proc_macro_attribute]
pub fn lapack(_attr: TokenStream, func: TokenStream) -> TokenStream {
    lapack2(syn::parse(func).unwrap()).into()
}

// TokenStream2-based main routine
fn lapack2(func: TokenStream2) -> TokenStream2 {
    let f: syn::ForeignItemFn = syn::parse2(func).unwrap();
    let _input = signature_input(&f.sig.inputs);
    let _call = call(&f.sig.inputs);
    quote! { #f }
}

/// Parse type ascription pattern `a: *mut f64` into ("a", "f64")
fn parse_input(pat: &syn::PatType) -> (String, String) {
    let name = match &*pat.pat {
        syn::Pat::Ident(ident) => ident.ident.to_string(),
        _ => unreachable!(),
    };
    let ty = match &*pat.ty {
        syn::Type::Ptr(ptr_ty) => match &*ptr_ty.elem {
            syn::Type::Path(path) => {
                if let Some(id) = path.path.get_ident() {
                    id.to_string()
                } else {
                    unreachable!()
                }
            }
            _ => unreachable!(),
        },
        _ => unreachable!("LAPACK raw API must be consists of pointer arguments"),
    };
    let ty = match ty.as_str() {
        "c_char" => "u8",
        "c_int" => "i32",
        ty => ty,
    };
    (name, ty.into())
}

/// Convert pointer-based raw-LAPACK API into value and reference based API
fn signature_input(args: &Args) -> Args {
    let mut args = args.clone();
    for arg in &mut args {
        match arg {
            syn::FnArg::Typed(arg) => {
                let (name, ty) = parse_input(arg);
                let new_type = match name.to_lowercase().as_str() {
                    // pointer -> mutable reference
                    "info" => "&mut i32".into(),
                    // pointer -> array
                    "a" | "b" | "ipiv" => format!("&mut [{}]", ty),
                    // pointer -> value
                    _ => ty.into(),
                };
                *arg.ty = syn::parse_str(&new_type).unwrap();
            }
            _ => unreachable!("LAPACK raw API does not contains non-typed argument"),
        }
    }
    args
}

fn call(args: &Args) -> Call {
    args.iter()
        .map(|arg| match arg {
            syn::FnArg::Typed(arg) => {
                let (name, _ty) = parse_input(arg);
                let expr = match name.to_lowercase().as_str() {
                    "info" => "info".into(),
                    "a" | "b" | "ipiv" => format!("{}.as_mut_ptr()", name),
                    _ => format!("&{}", name),
                };
                syn::parse_str::<syn::Expr>(&expr).unwrap()
            }
            _ => unreachable!(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_input() {
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
        let f: syn::ForeignItemFn = syn::parse_str(dgetrs).unwrap();
        let result = super::signature_input(&f.sig.inputs);
        let result_str = quote! { #result }.to_string();
        let answer: TokenStream2 = syn::parse_str(
            r#"
            trans: u8,
            n: i32,
            nrhs: i32,
            A: &mut [f64],
            lda: i32,
            ipiv: &mut [i32],
            B: &mut [f64],
            ldb: i32,
            info: &mut i32,
            "#,
        )
        .unwrap();
        assert_eq!(result_str, answer.to_string());
    }

    #[test]
    fn call() {
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
        let f: syn::ForeignItemFn = syn::parse_str(dgetrs).unwrap();
        let result = super::call(&f.sig.inputs);
        let result_str = quote! { #result }.to_string();
        let answer: TokenStream2 = syn::parse_str(
            r#"
            &(trans as c_char),
            &n,
            &nrhs,
            a.as_ptr(),
            &lda,
            ipiv.as_ptr(),
            b.as_mut_ptr(),
            &ldb,
            info,
            "#,
        )
        .unwrap();
        assert_eq!(result_str, answer.to_string());
    }

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
        let wrapped = lapack2(syn::parse_str(dgetrs).unwrap());
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
