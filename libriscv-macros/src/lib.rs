use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    parse_quote,
    spanned::Spanned,
    FnArg,
    GenericArgument,
    ItemFn,
    PathArguments,
    ReturnType,
    Type,
};

fn validate_context_arg(input: &ItemFn, expected: &str) -> Result<(), syn::Error> {
    if input.sig.inputs.len() != 1 {
        return Err(syn::Error::new(
            input.sig.span(),
            format!("expected exactly one argument: &mut {expected}"),
        ));
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
                            .is_some_and(|seg| seg.ident == expected),
                        _ => false,
                    }
                }
            }
            _ => false,
        },
        _ => false,
    };

    if ok {
        Ok(())
    } else {
        Err(syn::Error::new(
            input.sig.span(),
            format!("argument must be &mut {expected}"),
        ))
    }
}

fn wrap_syscall_result(input: &mut ItemFn, crate_path: &proc_macro2::TokenStream) {
    if let ReturnType::Type(_, ty) = &input.sig.output {
        if let Type::Path(type_path) = &**ty {
            if let Some(segment) = type_path.path.segments.last() {
                if segment.ident == "SyscallResult" {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(GenericArgument::Type(inner)) = args.args.first() {
                            let block = input.block.clone();
                            let inner = inner.clone();
                            input.block = Box::new(parse_quote!({
                                let result: #crate_path::Result<#inner> = (|| #block)();
                                result.into()
                            }));
                        }
                    }
                }
            }
        }
    }
}

/// Define a safe syscall handler and generate a `*_handler()` constructor.
///
/// The annotated function must be a non-`async`, non-`unsafe` function with
/// the signature `fn(&mut SyscallContext) -> ()` or
/// `fn(&mut SyscallContext) -> SyscallResult<T>`. The macro generates an
/// `extern "C"` trampoline that catches panics and forwards the return value
/// to `SyscallHandlerOutput`.
#[proc_macro_attribute]
pub fn syscall_handler(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new(Span::call_site(), "syscall_handler takes no arguments")
            .to_compile_error()
            .into();
    }

    let mut input = syn::parse_macro_input!(item as ItemFn);
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
    if let Err(err) = validate_context_arg(&input, "SyscallContext") {
        return err.to_compile_error().into();
    }

    let crate_path = quote!(::libriscv);
    wrap_syscall_result(&mut input, &crate_path);

    let fn_name = &input.sig.ident;
    let wrapper_name = format_ident!("{}_handler", fn_name);
    let trampoline_name = format_ident!("__libriscv_syscall_trampoline_{}", fn_name);
    let vis = &input.vis;

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

/// Define a safe error handler and generate a `*_handler()` constructor.
///
/// The annotated function must be a non-`async`, non-`unsafe` function with
/// the signature `fn(&mut ErrorContext) -> ()` or
/// `fn(&mut ErrorContext) -> SyscallResult<T>`. The generated trampoline
/// catches panics and forwards the return value to `SyscallHandlerOutput`.
#[proc_macro_attribute]
pub fn error_handler(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new(Span::call_site(), "error_handler takes no arguments")
            .to_compile_error()
            .into();
    }

    let mut input = syn::parse_macro_input!(item as ItemFn);
    if input.sig.asyncness.is_some() {
        return syn::Error::new(input.sig.span(), "error_handler does not support async fns")
            .to_compile_error()
            .into();
    }
    if input.sig.unsafety.is_some() {
        return syn::Error::new(input.sig.span(), "error_handler expects a safe function")
            .to_compile_error()
            .into();
    }
    if !input.sig.generics.params.is_empty() {
        return syn::Error::new(input.sig.span(), "error_handler does not support generics")
            .to_compile_error()
            .into();
    }
    if input.sig.variadic.is_some() {
        return syn::Error::new(input.sig.span(), "error_handler does not support variadics")
            .to_compile_error()
            .into();
    }
    if let Err(err) = validate_context_arg(&input, "ErrorContext") {
        return err.to_compile_error().into();
    }

    let crate_path = quote!(::libriscv);
    wrap_syscall_result(&mut input, &crate_path);

    let fn_name = &input.sig.ident;
    let wrapper_name = format_ident!("{}_handler", fn_name);
    let trampoline_name = format_ident!("__libriscv_error_trampoline_{}", fn_name);
    let vis = &input.vis;

    let expanded = quote! {
        #input

        #[doc(hidden)]
        unsafe extern "C" fn #trampoline_name(
            opaque: *mut ::std::os::raw::c_void,
            type_: ::std::os::raw::c_int,
            msg: *const ::std::os::raw::c_char,
            data: ::std::os::raw::c_long,
        ) {
            let _ = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| {
                let mut ctx = unsafe { #crate_path::ErrorContext::from_raw(opaque, type_, msg, data) };
                #crate_path::SyscallHandlerOutput::handle(#fn_name(&mut ctx));
            }));
        }

        #vis fn #wrapper_name() -> #crate_path::ErrorHandler {
            unsafe { #crate_path::ErrorHandler::new(#trampoline_name) }
        }
    };

    expanded.into()
}

/// Define a safe stdout handler and generate a `*_handler()` constructor.
///
/// The annotated function must be a non-`async`, non-`unsafe` function with
/// the signature `fn(&mut StdoutContext) -> ()` or
/// `fn(&mut StdoutContext) -> SyscallResult<T>`. The generated trampoline
/// catches panics and forwards the return value to `SyscallHandlerOutput`.
#[proc_macro_attribute]
pub fn stdout_handler(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new(Span::call_site(), "stdout_handler takes no arguments")
            .to_compile_error()
            .into();
    }

    let mut input = syn::parse_macro_input!(item as ItemFn);
    if input.sig.asyncness.is_some() {
        return syn::Error::new(input.sig.span(), "stdout_handler does not support async fns")
            .to_compile_error()
            .into();
    }
    if input.sig.unsafety.is_some() {
        return syn::Error::new(input.sig.span(), "stdout_handler expects a safe function")
            .to_compile_error()
            .into();
    }
    if !input.sig.generics.params.is_empty() {
        return syn::Error::new(input.sig.span(), "stdout_handler does not support generics")
            .to_compile_error()
            .into();
    }
    if input.sig.variadic.is_some() {
        return syn::Error::new(input.sig.span(), "stdout_handler does not support variadics")
            .to_compile_error()
            .into();
    }
    if let Err(err) = validate_context_arg(&input, "StdoutContext") {
        return err.to_compile_error().into();
    }

    let crate_path = quote!(::libriscv);
    wrap_syscall_result(&mut input, &crate_path);

    let fn_name = &input.sig.ident;
    let wrapper_name = format_ident!("{}_handler", fn_name);
    let trampoline_name = format_ident!("__libriscv_stdout_trampoline_{}", fn_name);
    let vis = &input.vis;

    let expanded = quote! {
        #input

        #[doc(hidden)]
        unsafe extern "C" fn #trampoline_name(
            opaque: *mut ::std::os::raw::c_void,
            msg: *const ::std::os::raw::c_char,
            len: ::std::os::raw::c_uint,
        ) {
            let _ = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| {
                let mut ctx = unsafe { #crate_path::StdoutContext::from_raw(opaque, msg, len) };
                #crate_path::SyscallHandlerOutput::handle(#fn_name(&mut ctx));
            }));
        }

        #vis fn #wrapper_name() -> #crate_path::StdoutHandler {
            unsafe { #crate_path::StdoutHandler::new(#trampoline_name) }
        }
    };

    expanded.into()
}
