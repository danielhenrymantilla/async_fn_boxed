#![allow(nonstandard_style, unused_imports, unused_braces)]

use ::core::mem;
use ::proc_macro::TokenStream;
use ::proc_macro2::{TokenStream as TokenStream2, *};
use ::quote::{format_ident, quote, quote_spanned, ToTokens};
use ::syn::{
    parse::{Parse, ParseStream, Parser},
    punctuated::Punctuated,
    spanned::Spanned,
    Result, // explicitly shadow it.
    *,
};

mod lifetime_elision;

#[rustfmt::skip]
macro_rules! bail {
    (
        $error_message:expr $(,)?
    ) => (
        return Err(Error::new(Span::mixed_site(), format!($error_message)))
    );

    (
        $error_message:expr => $spanned:expr $(,)?
    ) => (
        return Err(Error::new_spanned(&$spanned, format!($error_message)))
    );
}

#[proc_macro_attribute]
pub fn async_fn_boxed(args: TokenStream, input: TokenStream) -> TokenStream {
    async_fn_boxed_inner(args.into(), input.into())
        /* coarse debugging */
        // .map(|tts| { println!("{tts}"); tts })
        /* Prefix error message with ourselves */
        .map_err(|err| {
            Error::new_spanned(&err.to_compile_error(), format!("#[async_fn_boxed]: {err}"))
        })
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn async_fn_boxed_inner(args: TokenStream2, input: TokenStream2) -> Result<TokenStream2> {
    let _args: parse::Nothing = parse2(args)?;
    let mut fun: ItemFn = parse2(input)?;
    // 1. extract (and remove) the `async` from `async fn`.
    let Some(async_) = mem::take(&mut fun.sig.asyncness) else {
        bail! {
            "expected an `async fn`" => fun.sig.fn_token,
        }
    };
    // 2. extract the `Ret`urn type to wrap in `Pin<Box<dyn …>>`.
    // Produce the `'fut` lifetime
    let fut_lt: Lifetime = lifetime_of_future_of_async_fn(&mut fun, async_.span())?;
    let Ret: Type = match &fun.sig.output {
        ReturnType::Default => parse_quote_spanned!(fun.block.brace_token.span.open()=>
            ()
        ),
        ReturnType::Type(_arrow, ty) => (**ty).clone(),
    };
    let ActualRet: Type = parse_quote!(
        ::core::pin::Pin<
            ::std::boxed::Box<
                dyn #fut_lt + ::core::marker::Send + ::core::future::Future<
                    Output = #Ret,
                >,
            >,
        >
    );
    fun.sig.output = parse_quote_spanned!(Ret.span()=>
        -> #ActualRet
    );
    let block = &mut *fun.block;
    block.stmts.insert(
        0,
        parse_quote_spanned!(Ret.span()=>
            if false { return ::core::option::Option::<#Ret>::None {}.unwrap(); }
        ),
    );
    // finally, transform block = `{ ... body }` into:
    // ```
    // { // <- outer
    //     Box::pin(async move { // <- inner
    //         ... body
    //     })
    // }
    // ```
    let brace_token = block.brace_token.clone();
    *block = parse_quote_spanned!(brace_token.span=>
        {
            ::std::boxed::Box::pin(
                async move #block
            )
        }
    );
    Ok(fun.into_token_stream())
}

/// Given an `async fn` such as:
///
/// ```rs
/// # struct EachArg; struct Ret;
/// async fn foo</* generics */>(each_arg: EachArg, /* … */) -> Ret {
///     // …
///     Ret
/// }
/// ```
///
/// the desired lifetime unsugaring is:
///
/// ```rs
/// //     👇👇
/// fn foo<'fut, 'each, Generic /*, … */>(/* … */) -> BoxFuture<'fut, Ret>
/// where
///     'each   : 'fut, // 👈
///     Generic : 'fut, // 👈
///     // …            // 👈
/// {
///     async move {
///         let _captured = (&each_arg, /* … */);
///         // …
///         Ret
///     }
/// }
/// ```
fn lifetime_of_future_of_async_fn(fun: &mut ItemFn, async_span: Span) -> Result<Lifetime> {
    // 0. Unelide implicit lifetimes (in input position only).
    lifetime_elision::unelide_input_lifetimes(&mut fun.sig)?;
    // 1. Create the new lifetime param token.
    let fut_lt = Lifetime::new("'__fut", async_span);
    // 2. Add the `where` clauses.
    let where_predicates = &mut fun
        .sig
        .generics
        .where_clause // same as `.make_where_clause` but for a finer-grained borrow.
        .get_or_insert_with(|| parse_quote!(where))
        .predicates;
    for generic in &fun.sig.generics.params {
        let generic: &dyn ToTokens = match generic {
            GenericParam::Lifetime(LifetimeParam { lifetime, .. }) => lifetime,
            GenericParam::Type(TypeParam { ident: Ty, .. }) => Ty,
            GenericParam::Const(_) => continue, // constinue?
        };
        where_predicates.push(parse_quote!(
            #generic : #fut_lt
        ));
    }
    // 3. Add it to the generics:
    introduce_new_generic_lt(&fut_lt, &mut fun.sig.generics);
    Ok(fut_lt)
}

fn introduce_new_generic_lt(lifetime: &Lifetime, generics: &mut Generics) {
    generics
        .params
        .insert(generics.lifetimes().count(), parse_quote!( #lifetime ));
}
