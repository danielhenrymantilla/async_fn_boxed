use super::*;

use ::syn::visit_mut::{self as subrecurse, VisitMut};

pub(crate) fn unelide_input_lifetimes(sig: &mut Signature) -> Result<()> {
    // unelide the inputs:
    let mut i = 0..;
    let mut visitor = ElidedLifetimesVisitor {
        map_each_elided_lt: |elided_lifetime_span| -> Option<Lifetime> {
            let mut candidate_name;
            while {
                // do:
                candidate_name = format!("'__{}", i.next().unwrap());
                // while:
                sig.generics
                    .lifetimes()
                    .any(|lt| lt.lifetime.ident == candidate_name)
            } {}
            let new_lifetime = Lifetime::new(&candidate_name, elided_lifetime_span);
            introduce_new_generic_lt(&new_lifetime, &mut sig.generics);
            Some(new_lifetime)
        },
    };
    sig.inputs
        .iter_mut()
        .for_each(|it| visitor.visit_fn_arg_mut(it));
    // No need to unelide the output: elided it will still Just Work™.
    Ok(())
}

struct ElidedLifetimesVisitor<F: FnMut(Span) -> Option<Lifetime>> {
    map_each_elided_lt: F,
}

impl<F: FnMut(Span) -> Option<Lifetime>> VisitMut for ElidedLifetimesVisitor<F> {
    /// `'_` case.
    fn visit_lifetime_mut(&mut self, lt: &mut Lifetime) {
        if lt.ident == "_" {
            if let Some(new_lt) = (self.map_each_elided_lt)(lt.span()) {
                *lt = new_lt;
            }
        }
    }

    /// `& /* no lifetime */ [mut]? <ty>` case.
    fn visit_type_reference_mut(&mut self, ref_: &mut TypeReference) {
        // 1. subrecurse into `Lifetime`, if any, and into referee `Type`.
        subrecurse::visit_type_reference_mut(self, ref_);
        // 2. Handle the case with *no* `Lifetime`.
        if ref_.lifetime.is_none() {
            ref_.lifetime = (self.map_each_elided_lt)(ref_.and_token.span());
        }
    }

    // == Exceptions: `fn(…) -> …` and `Fn{,Mut,Once}(…) -> …` ==
    // For instance, it would be incorrect to transform `fn foo(f: fn(&str))`
    // into `fn foo<'elided>(f: fn(&'elided str))`, since the actual meaning is
    // `fn foo(f: for<'elided> fn(&'elided) str)`.

    /// `fn(…) -> …`
    fn visit_type_bare_fn_mut(&mut self, _: &mut TypeBareFn) {
        /* do nothing: mainly, do not `subrecurse` */
    }

    /// `Fn{,Mut,Once}(…) -> …`
    fn visit_parenthesized_generic_arguments_mut(&mut self, _: &mut ParenthesizedGenericArguments) {
        /* do nothing: mainly, do not `subrecurse` */
    }
}
