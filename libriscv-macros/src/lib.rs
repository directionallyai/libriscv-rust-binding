use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{spanned::Spanned, FnArg, ItemFn, Type};

#[proc_macro_attribute]
pub fn syscall_handler(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new(Span::call_site(), "syscall_handler takes no arguments")
            .to_compile_error()
            .into();
    }

    let input = syn::parse_macro_input!(item as ItemFn);
    if input.sig.asyncness.is_some() {
        return syn::Error::new(input.sig.span(), "syscall_handler does not support async fns")
            .to_compile_error()
            .into();
    }
    if input.sig.unsafety.is_some() {
        return syn::Error::new(input.sig.span(), "syscall_handler expects a safe function")
            .to_compile_error()
            .into();
    }
    if !input.sig.generics.params.is_empty() {
        return syn::Error::new(input.sig.span(), "syscall_handler does not support generics")
            .to_compile_error()
            .into();
    }
    if input.sig.variadic.is_some() {
        return syn::Error::new(input.sig.span(), "syscall_handler does not support variadics")
            .to_compile_error()
            .into();
    }
    if input.sig.inputs.len() != 1 {
        return syn::Error::new(
            input.sig.span(),
            "syscall_handler expects exactly one argument: &mut SyscallContext",
        )
        .to_compile_error()
        .into();
    }

    let arg = input.sig.inputs.first().expect("checked len");
    let ok = match arg {
        FnArg::Typed(pat) => match &*pat.ty {
            Type::Reference(reference) => {
                if reference.mutability.is_none() {
                    false
                } else {
                    match &*reference.elem {
                        Type::Path(path) => path
                            .path
                            .segments
                            .last()
                            .is_some_and(|seg| seg.ident == "SyscallContext"),
                        _ => false,
                    }
                }
            }
            _ => false,
        },
        _ => false,
    };

    if !ok {
        return syn::Error::new(
            input.sig.span(),
            "syscall_handler argument must be &mut SyscallContext",
        )
        .to_compile_error()
        .into();
    }

    let fn_name = &input.sig.ident;
    let wrapper_name = format_ident!("{}_handler", fn_name);
    let trampoline_name = format_ident!("__libriscv_syscall_trampoline_{}", fn_name);
    let vis = &input.vis;

    let crate_path = quote!(::libriscv);

    let expanded = quote! {
        #input

        #[doc(hidden)]
        unsafe extern "C" fn #trampoline_name(m: *mut #crate_path::sys::RISCVMachine) {
            let _ = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| {
                if let Some(mut ctx) = unsafe { #crate_path::SyscallContext::from_raw(m) } {
                    #crate_path::SyscallHandlerOutput::handle(#fn_name(&mut ctx));
                }
            }));
        }

        #vis fn #wrapper_name() -> #crate_path::SyscallHandler {
            unsafe { #crate_path::SyscallHandler::new(#trampoline_name) }
        }
    };

    expanded.into()
}
