use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{DeriveInput, Ident};

use crate::common::{add_trait_bound, combined_error, parse_generics, ParsedGenerics};

pub fn bounds_impl(item: &DeriveInput) -> TokenStream {
    let bounds_ident = &item.ident;

    let generics = add_trait_bound(&item.generics, quote!(num::cast::ToPrimitive));
    let ParsedGenerics {
        generic_param,
        impl_generics,
        type_generics,
        where_clause,
    } = match parse_generics(&generics) {
        Ok(v) => v,
        Err(errors) => {
            return combined_error("Impl Point", item.ident.span(), errors).to_compile_error();
        }
    };

    let bounds_str = bounds_ident.to_string();
    let component_count = match bounds_str.chars().last().unwrap().to_digit(10) {
        Some(c) => c,
        None => {
            return syn::Error::new_spanned(
                &item.ident,
                "Impl 'Bounds': Expected component count at the end of name.",
            )
            .to_compile_error()
        }
    };
    let point_ident = Ident::new(&format!("Point{}", component_count), Span::call_site());
    let point_str = point_ident.to_string();
    let vec_ident = Ident::new(&format!("Vec{}", component_count), Span::call_site());
    let vec_str = vec_ident.to_string();

    let bounds_type = quote!(#bounds_ident #type_generics);
    let point_type = quote!(#point_ident #type_generics);
    let vec_type = quote!(#vec_ident #type_generics);

    let new_doc = format!(
        "Creates a new `{0}` around the two [{1}]s.",
        bounds_str, point_str
    );
    let default_doc = format!(
        "Creates a new `{0}` with minimum extents at [T::max_value](num::Bounded::max_value) and maximum extents at [T::min_value](num::Bounded::min_value).",
        bounds_str
    );
    let diagonal_doc = format!("Returns the [{0}] from `p_min` to `p_max`.", vec_str);
    let union_b_doc = format!(
        "Creates a new `{0}` that encompasses this `{0}` and another `{0}`.",
        bounds_str
    );
    let union_p_doc = format!(
        "Creates a new `{0}` that encompasses this `{0}` and a [{1}].",
        bounds_str, point_str
    );
    let intersection_doc = format!(
        "Creates a new `{0}` that is the bounding box of the intersection of this `{0}` and another `{0}`.\n\
        \n\
        Returns [None] if no intersection exists.",
        bounds_str
    );
    let overlaps_doc = format!("Checks if this `{0}` overlaps another `{0}`.", bounds_str);
    let overlaps_pred = per_component_tokens(
        component_count,
        &|c: &Ident| quote!((self.p_max.#c >= other.p_min.#c) && (self.p_min.#c <= other.p_max.#c)),
        &|recurse| quote!(#(#recurse)&&*),
    );
    let inside_doc = format!(
        "Checks if a [{0}] is inside this `{1}`.",
        point_str, bounds_str
    );
    let inside_pred = per_component_tokens(
        component_count,
        &|c: &Ident| quote!((p.#c >= self.p_min.#c) && (p.#c <= self.p_max.#c)),
        &|recurse| quote!(#(#recurse)&&*),
    );
    let inside_exclusive_doc = format!(
        "Checks if a [{0}] is inside this `{1}`, excluding the upper bound.",
        point_str, bounds_str
    );
    let inside_exclusive_pred = per_component_tokens(
        component_count,
        &|c: &Ident| quote!((p.#c >= self.p_min.#c) && (p.#c < self.p_max.#c)),
        &|recurse| quote!(#(#recurse)&&*),
    );
    let expanded_doc = format!(
        "Creates a new `{0}` with this `{0}` expanded by `delta` in all directions.\n\
        \n\
        Returns [None] if a negative delta would have caused `p_min > p_max` for any component.",
        bounds_str
    );
    let expanded_pred = per_component_tokens(
        component_count,
        &|c: &Ident| quote!((p_min.#c <= p_max.#c)),
        &|recurse| quote!(#(#recurse)&&*),
    );
    let lerp_doc = format!(
        "Linearly interpolates between the corners of this `{0}` by the components of `t`.",
        bounds_str
    );
    let lerp_components = per_component_tokens(
        component_count,
        // We assume any Num type can be converted to and from f32
        &|c: &Ident| {
            quote! {
                #generic_param::from_f32(
                    (1.0 - t.#c) * self.p_min.#c.to_f32().unwrap() + t.#c * self.p_max.#c.to_f32().unwrap()
                ).unwrap()
            }
        },
        &|recurse| quote!(#(#recurse),*),
    );
    let offset_doc = format!(
        "Calculates the relative position of a [{0}] from the corners of this `{1}`. The lower bound maps to `(0,0,0)` and the upper to `(1,1,1)`.",
        point_str,
        bounds_str
    );
    let offset_scales = per_component_tokens(
        component_count,
        &|c: &Ident| quote!(if (self.p_max.#c != self.p_min.#c) { o.#c /= self.p_max.#c - self.p_min.#c; }),
        &|recurse| quote!(#(#recurse)*),
    );

    quote! {
        impl #impl_generics #bounds_type
        #where_clause
        {
            #[doc = #new_doc]
            #[inline]
            pub fn new(p0: #point_type, p1: #point_type) -> Self {
                Self {
                    p_min: p0.min(p1),
                    p_max: p0.max(p1),
                }
            }

            #[doc = #default_doc]
            #[inline]
            pub fn default() -> Self {
                Self {
                    p_min: #point_ident::from(<#generic_param>::max_value()),
                    p_max: #point_ident::from(<#generic_param>::min_value()),
                }
            }

            #[doc = #union_b_doc]
            #[inline]
            pub fn union_b(&self, other: Self) -> Self {
                debug_assert!(!self.p_min.has_nans() && !self.p_max.has_nans());
                debug_assert!(!other.p_min.has_nans() && !other.p_max.has_nans());

                Self {
                    p_min: self.p_min.min(other.p_min),
                    p_max: self.p_max.max(other.p_max),
                }
            }

            #[doc = #union_p_doc]
            #[inline]
            pub fn union_p(&self, p: #point_type) -> Self {
                debug_assert!(!self.p_min.has_nans() && !self.p_max.has_nans());
                debug_assert!(!p.has_nans());

                Self {
                    p_min: self.p_min.min(p),
                    p_max: self.p_max.max(p),
                }
            }

            #[doc = #intersection_doc]
            #[inline]
            pub fn intersection(&self, other: Self) -> Option<Self> {
                debug_assert!(!self.p_min.has_nans() && !self.p_max.has_nans());
                debug_assert!(!other.p_min.has_nans() && !other.p_max.has_nans());

                // TODO: Is there a case where we don't want to pay for the check here?
                if self.overlaps(other) {
                    Some(
                        Self {
                            p_min: self.p_min.max(other.p_min),
                            p_max: self.p_max.min(other.p_max),
                        }
                    )
                } else {
                    None
                }
            }

            #[doc = #expanded_doc]
            #[inline]
            pub fn expanded(&self, delta: #generic_param) -> Option<Self> {
                debug_assert!(!self.p_min.has_nans() && !self.p_max.has_nans());
                // Not all types have is_nan() so use NaN != NaN
                debug_assert!(delta == delta);


                let p_min = self.p_min - #vec_ident::from(delta);
                let p_max = self.p_max + #vec_ident::from(delta);
                if #expanded_pred {
                    Some(
                        Self { p_min, p_max, }
                    )
                } else {
                    None
                }
            }

            #[doc = #overlaps_doc]
            #[inline]
            pub fn overlaps(&self, other: Self) -> bool {
                debug_assert!(!self.p_min.has_nans() && !self.p_max.has_nans());
                debug_assert!(!other.p_min.has_nans() && !other.p_max.has_nans());

                #overlaps_pred
            }

            #[doc = #inside_doc]
            #[inline]
            pub fn inside(&self, p: #point_type) -> bool {
                debug_assert!(!self.p_min.has_nans() && !self.p_max.has_nans());
                debug_assert!(!p.has_nans());

                #inside_pred
            }

            #[doc = #inside_exclusive_doc]
            #[inline]
            pub fn inside_exclusive(&self, p: #point_type) -> bool {
                debug_assert!(!self.p_min.has_nans() && !self.p_max.has_nans());
                debug_assert!(!p.has_nans());

                #inside_exclusive_pred
            }

            #[doc = #diagonal_doc]
            #[inline]
            pub fn diagonal(&self) -> #vec_type {
                self.p_max - self.p_min
            }

            #[doc = #lerp_doc]
            #[inline]
            // We assume we don't care about accuracy so much we would need f64
            pub fn lerp(&self, t: #point_ident<f32>) -> #point_type {
                #point_ident::new(#lerp_components)
            }

            #[doc = #offset_doc]
            #[inline]
            pub fn offset(&self, p: #point_type) -> #vec_type {
                let mut o = p - self.p_min;
                #offset_scales
                o
            }
        }
    }
}

type ComponentStreams<'a> = std::iter::Map<std::ops::Range<u32>, &'a dyn Fn(u32) -> TokenStream>;

fn per_component_tokens(
    component_count: u32,
    component_tokens: &dyn Fn(&Ident) -> TokenStream,
    combined_tokens: &dyn Fn(ComponentStreams) -> TokenStream,
) -> TokenStream {
    let components = [
        Ident::new("x", Span::call_site()),
        Ident::new("y", Span::call_site()),
        Ident::new("z", Span::call_site()),
    ];
    combined_tokens((0..component_count).map(&(|c| component_tokens(&components[c as usize]))))
}
