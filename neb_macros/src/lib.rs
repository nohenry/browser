use quote::{format_ident, quote};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    iter::repeat, fmt::Debug,
};
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(EnumHash)]
pub fn gen_hash(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // let tokens = TokenStream::from(item);
    let input = parse_macro_input!(item as DeriveInput);

    let variants = match &input.data {
        syn::Data::Enum(enum_item) => enum_item.variants.iter().map(|v| &v.ident),
        _ => panic!("AllVariants only works on enums"),
    };
    let enum_name = input.ident;

    let (varient_names, hashes): (Vec<_>, Vec<_>) = variants
        .map(|var| (var, calculate_hash(&to_camel(var.to_string()))))
        .unzip();

    let modname = format_ident!("{}Hashes", enum_name);

    // let enum_name = repeat(enum_name.to_string().as_str());

    let tokens = quote! {
        #[allow(non_snake_case, non_upper_case_globals)]
        pub mod #modname {
            #(pub const #varient_names: u64 = #hashes;
             )*
        }
    };
    tokens.into()
}

fn calculate_hash<T>(t: &T) -> u64
where
    T: Hash + Debug,
{
    let mut state = DefaultHasher::new();
    println!("Hashing: {:?}", t);
    t.hash(&mut state);
    state.finish()
}

fn to_camel(st: String) -> String {
    let mut it = st.chars();
    let first = it.next().unwrap().to_lowercase();
    first.chain(it).collect()
}