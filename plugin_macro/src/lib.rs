extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, FnArg, ItemFn, Pat, ReturnType, Stmt};

fn translate_inputs<'a>(it: impl Iterator<Item = &'a mut FnArg>) -> Vec<Stmt> {
    let preprocess_block: Stmt = parse_quote! {
        let args: jintemplify_plugin::InputWrapper = {
            let slice = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
            let json_str = match std::str::from_utf8(slice) {
                Ok(s) => s,
                Err(_) => {
                    return jintemplify_plugin::serialize_to_return_values(&jintemplify_plugin::ErrorValue {
                        reason: "Failed to convert byte slice to string".to_string(),
                    })
                }
            };
            let args = match serde_json::from_str(json_str) {
                Ok(val) => val,
                Err(err) => {
                    return jintemplify_plugin::serialize_to_return_values(&jintemplify_plugin::ErrorValue {
                        reason: format!("Failed to deserialize JSON: {}", err).to_string(),
                    })
                }
            };
            args
        };
    };

    let mut out: Vec<Stmt> = vec![preprocess_block];

    it.enumerate()
        .map(|(i, arg)| {
            let FnArg::Typed(arg) = arg else {
                panic!("self is not allowed for plugin functions")
            };
            let Pat::Ident(id) = &*arg.pat else {
                panic!("Invalid function declation")
            };
            (i, id.ident.clone(), &mut arg.ty)
        })
        .for_each(|(index, name, ty)| {
            out.push(
                parse_quote!(let #name: #ty = jintemplify_plugin::convert_value::<#ty>(&args.params[#index], #index).unwrap();),
            );
        });

    out
}

fn translate_output(ret: &mut ReturnType) -> Stmt {
    let mut out = parse_quote!(return (out.ptr, out.len););

    if let ReturnType::Type(_, _ty) = ret {
        out = parse_quote!({
            let out = jintemplify_plugin::OutputWrapper {
                result: serde_json::json!(out),
            };
            return jintemplify_plugin::serialize_to_return_values(&out);
        });
    }

    out
}

fn make_plugin(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut item_fn = parse_macro_input!(item as ItemFn);

    let prelude = translate_inputs(item_fn.sig.inputs.iter_mut());
    let epilode = translate_output(&mut item_fn.sig.output);

    let fn_name = &item_fn.sig.ident;
    let fn_block = &item_fn.block;
    let output_type = item_fn.sig.output.clone();

    let expanded = quote! {
        #[no_mangle]
        pub unsafe extern "C" fn #fn_name (ptr: *mut u8, len: u32) -> *mut jintemplify_plugin::ReturnValues {
            #(#prelude)*
            let out = (move || #output_type #fn_block)();
            #epilode
        }
    };

    expanded.into()
}

#[proc_macro_attribute]
pub fn plugin_filter(attr: TokenStream, item: TokenStream) -> TokenStream {
    make_plugin(attr, item)
}

#[proc_macro_attribute]
pub fn plugin_function(attr: TokenStream, item: TokenStream) -> TokenStream {
    make_plugin(attr, item)
}
