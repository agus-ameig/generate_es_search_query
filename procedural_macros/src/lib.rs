use darling::{ast::NestedMeta, FromMeta};
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use regex::Regex;
use serde_json::Value;
use syn::{
    parse::Parse, parse_macro_input, Data, DeriveInput, Fields, LitStr,
};

struct SearchQuery {
    query_str: LitStr
}

impl Parse for SearchQuery {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let query_str: LitStr  = input.parse()?;
        Ok(SearchQuery { query_str })

    }
}

fn validate_query_str_is_json(query_str: String) -> Result<Value, serde_json::Error> {
    let regex_for_str = Regex::new(r#"\"\{\{.+?\}\}\""#).unwrap();
    let regex_for_rest = Regex::new(r#"\{\{.+?\}\}"#).unwrap();
    let modified_query_str = regex_for_str.replace_all(query_str.as_str(), "\"a\"");
    let modified_query_str_2 = regex_for_rest.replace_all(modified_query_str.as_ref(), "1");
    serde_json::from_str::<Value>(&modified_query_str_2.to_string())
}

fn remove_whitespace(input: &str) -> String {
    input.split_whitespace().collect()
}

#[proc_macro_attribute]
pub fn create_search_query(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let input_name = &input.ident;

    let args = parse_macro_input!(args as SearchQuery);
    let query_str = remove_whitespace(&args.query_str.value());

    match validate_query_str_is_json(query_str.clone()) {
        Ok(_) => {},
        Err(e) => return syn::Error::new(
            args.query_str.span(),
            format!("Provided JSON is invalid: {}", e.to_string())
        ).to_compile_error().into()
    }

    let fields = match input.data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => fields.named.iter().map(|f| &f.ident).collect::<Vec<_>>(),
                _ => return syn::Error::new(input_name.span(), "Only named fields are supported").to_compile_error().into(),
            }
        }
        _ => return syn::Error::new(input_name.span(), "Only structs are supported").to_compile_error().into(),
    };

    let expanded = quote! {
        #input

        impl #input_name {
            pub fn get_search_query(&self) -> String {
                let mut query = String::from(#query_str);
                #(
                    let escaped_value = serde_json::to_string(&self.#fields.to_string())
                                        .unwrap_or_else(|_| String::from("\"\""))
                                        .trim_matches('"')
                                        .to_string();
                    query = query.replace(
                        &format!("{{{{{}}}}}", stringify!(#fields)),
                        &escaped_value,
                    );
                )*
                query
           }
        }
    };
    TokenStream::from(expanded)
}

#[derive(Debug,Clone)]
struct Searchable (Vec<LitStr>);

#[derive(FromMeta,Debug,Clone)]
enum FilterType {
   Range {
       name: syn::Ident,
       data_type: syn::Ident,
       #[darling(default)]
       gte: Option<syn::Ident>,
       #[darling(default)]
       lte: Option<syn::Ident>
   },
   Terms { name: syn::Ident, data_type: syn::Ident },
   MultiMatch { query: syn::Ident, data_type: syn::Ident, fields: Searchable}
}

impl FromMeta for Searchable {
    fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
        if let syn::Meta::List(meta_list) = item {
            let tokens = meta_list.clone().tokens;
            let list: Searchable = syn::parse2::<Searchable>(tokens).expect("Unable to parse list");
            Ok(list)
        } else if let syn::Meta::NameValue(name_value) = item {
            let mut tokens = name_value.clone().value.to_token_stream().into_iter();
            tokens.next();
            let remaining_tokens = if let Some(proc_macro2::TokenTree::Group(group)) = tokens.next() {
                    group.stream()
            } else { proc_macro2::TokenStream::new() };
            let list: Searchable = syn::parse2::<Searchable>(remaining_tokens).expect("Unable to parse list");
            Ok(list)
        }
        else {
            Err(darling::Error::custom("Expected a list of string"))
        }
    }
}

impl Parse for Searchable {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut searchable: Vec<LitStr> = vec![];

        while !input.is_empty() {
            let entry = input.parse::<LitStr>()?;
            if !input.is_empty() {
                input.parse::<syn::Token![,]>()?;
            }
            searchable.push(entry);
        }
        Ok(Searchable(searchable))
    }
}

#[derive(FromMeta,Clone,Debug)]
struct SearchArguments {
    #[darling(multiple, rename = "filter")]
    filters: Vec<FilterType>
}



#[proc_macro_attribute]
pub fn generate_search_query(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let parsed_args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(v) => v,
        Err(e) => {
            return darling::Error::from(e).write_errors().into();
        }
    };
    let search_args = SearchArguments::from_list(&parsed_args).expect("Cannot parse Arguments");
    let input_name = &input.ident;
    let search_query = build_search_query(&search_args);

    quote! {
         #input
         impl generate_es_search_query::SearchQuery for #input_name {
            fn get_search_query(&self) -> generate_es_search_query::Query {
                let mut filters: Vec<generate_es_search_query::ClauseType> = vec![];
                #(#search_query)*
                generate_es_search_query::Query {
                    query: generate_es_search_query::ClauseType::String(generate_es_search_query::ClauseFilter::<String>::BoolQuery(generate_es_search_query::BoolQuery {
                        must: filters
                    }))
                }
            }
         }
    }.into()
}

fn build_search_query(search_args: &SearchArguments) -> Vec<proc_macro2::TokenStream> {
    search_args.filters.iter().map(|filter_type| {
        match filter_type {
            FilterType::MultiMatch { query, data_type, fields } => {
                let fields_vec = fields.0.iter();
                quote! {
                    filters.push(generate_es_search_query::ClauseType::#data_type(
                       generate_es_search_query::ClauseFilter::<#data_type>::MultiMatch(generate_es_search_query::MultiMatch {
                            fields: vec![#(#fields_vec),*],
                            query: self.#query.clone()
                        })
                    ));
                }
            },

            FilterType::Range { name, data_type, gte, lte } => {
                let gte = if let Some(g) = gte {
                    quote! {
                        gte: &self.#g,
                    }
                } else { quote! {} };
                let lte = if let Some(l) = lte {
                    quote! {
                        lte: &self.#l,
                    }
                } else { quote! {} };

                quote! {
                    filters.push(generate_es_search_query::ClauseType::#data_type(
                       generate_es_search_query::ClauseFilter::<#data_type>::Range(RangeFilter {
                            field: stringify!(#name),
                            #gte
                            #lte
                        })
                    ));
                }

            },
            FilterType::Terms { name, data_type } => {
                quote! {
                    filters.push(generate_es_search_query::ClauseType::#data_type(
                       generate_es_search_query::ClauseFilter::<#data_type>::Terms(generate_es_search_query::TermsFilter {
                            field: stringify!(#name).to_string(),
                            terms_list: self.#name.clone()
                        })
                    ));
                }
            }
        }
    }).collect::<Vec<_>>()
}
