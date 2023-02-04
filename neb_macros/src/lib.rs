use quote::{format_ident, quote};
use std::{
    collections::hash_map::DefaultHasher,
    fmt::Debug,
    hash::{Hash, Hasher},
};
use syn::{parse_macro_input, DeriveInput, Fields};

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

#[proc_macro_derive(EnumExtract)]
pub fn extract(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // let tokens = TokenStream::from(item);
    let input = parse_macro_input!(item as DeriveInput);

    let (variants, fields): (Vec<_>, Vec<_>) = match input.data {
        syn::Data::Enum(enum_item) => enum_item
            .variants
            .into_iter()
            .filter_map(|v| match v.fields {
                Fields::Named(f) => Some((
                    v.ident,
                    f.named
                        .into_iter()
                        .map(|f| f.ident.unwrap())
                        .collect::<Vec<_>>(),
                )),
                _ => None,
            })
            .unzip(),
        _ => panic!("AllVariants only works on enums"),
    };
    let enum_name = input.ident;

    // let (varient_names, hashes): (Vec<_>, Vec<_>) = variants
    //     .map(|var| (var, calculate_hash(&to_camel(var.to_string()))))
    //     .unzip();

    let macro_name = format_ident!("{}As", enum_name);
    println!("{:?}", fields);

    // let enum_name = repeat(enum_name.to_string().as_str());

    let tokens = quote! {
    //     #[allow(non_snake_case, non_upper_case_globals)]
    //     pub mod #macro_name {
    //         #(pub const #variants: &[&str] = &[#(#fields,)*];
    //     )*
    //     }
    #[macro_export]
    macro_rules! #macro_name {
        #(($e:expr, #variants) => {
            match $e {
                #enum_name::#variants { #(#fields),* } => Some((#(#fields),*)),
                _ => None,
            }
        };)*
    }
        };
    tokens.into()
}

// #[derive(Debug)]
// struct ExtractEnum {
//     init: Expr,
//     enum_type: Ident,
//     enum_varient: Ident,
// }

// impl Parse for ExtractEnum {
//     fn parse(input: ParseStream) -> syn::Result<Self> {
//         // let visibility: Visibility = input.parse()?;
//         // input.parse::<Token![static]>()?;
//         // input.parse::<Token![ref]>()?;
//         // let name: Ident = input.parse()?;
//         // input.parse::<Token![:]>()?;
//         // let ty: Type = input.parse()?;
//         // input.parse::<Token![=]>()?;
//         let init: Expr = input.parse()?;
//         input.parse::<Token![,]>()?;
//         let enum_type: Ident = input.parse()?;
//         input.parse::<Token![::]>()?;
//         let enum_varient: Ident = input.parse()?;

//         Ok(ExtractEnum {
//             init,
//             enum_type,
//             enum_varient,
//         })
//     }
// }

// #[proc_macro]
// pub fn extract_enum(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
//     let ExtractEnum {
//         init,
//         enum_type,
//         enum_varient,
//     } = parse_macro_input!(input as ExtractEnum);
//     println!("input: {:?}", input);

//     quote! {
//         match #init {

//         }
//     }

//     proc_macro::TokenStream::new()
// }

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
