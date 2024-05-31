use darling::{ast, util::Flag, util::SpannedValue, FromDeriveInput, FromField};
use heck::AsShoutySnekCase;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{parse_macro_input, ExprPath};

#[derive(Debug, FromDeriveInput)]
#[darling(supports(struct_named))]
struct ServiceConfigReceiver {
	ident: syn::Ident,
	generics: syn::Generics,
	data: ast::Data<(), SpannedValue<ServiceConfigField>>,
}

impl ToTokens for ServiceConfigReceiver {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let struct_name = &self.ident;
		let (imp, ty, wher) = self.generics.split_for_impl();

		let mut fields_tokens = TokenStream::new();
		let mut sensitive_env_vars_tokens = TokenStream::new();

		#[allow(clippy::expect_used)] // Ensured by darling(supports(struct_named))
		for f in self
			.data
			.as_ref()
			.take_struct()
			.expect("data to be a struct")
			.fields
		{
			#[allow(clippy::expect_used)] // Ensured by darling(supports(struct_named))
			let field_name = f.ident.as_ref().expect("named field to have a name");
			let field_name_as_string = field_name.to_string();
			let env_var_format_string = format!(
				"{{}}_{shouty_field_name}",
				shouty_field_name = AsShoutySnekCase(&field_name_as_string),
			);
			let field_type = &f.ty;

			let (is_option, value_type) = if let Some(value_type) = optionalise_type(field_type) {
				(true, value_type)
			} else {
				(false, field_type)
			};

			let value_parser = if let Some(value_parser) = &f.value_parser {
				let parser = value_parser.as_ref();
				quote_spanned! { value_parser.span()=> #parser }
			} else {
				quote_spanned! { f.span()=>
					|s: &str| s.parse::<#value_type>()
				}
			};

			let default_value = if let Some(default_value) = &f.default_value {
				let value = default_value.as_ref();
				quote_spanned! { default_value.span()=> Some(#value) }
			} else {
				quote_spanned! { f.span()=> None }
			};

			if f.is_sensitive() {
				sensitive_env_vars_tokens.extend(quote! {
					std::env::remove_var(&format!(#env_var_format_string, prefix));
				});
			}

			if is_option {
				fields_tokens.extend(quote! {
					#field_name: service_skeleton::config::determine_optional_value(&format!(#env_var_format_string, prefix), #value_parser, map.get(&format!(#env_var_format_string, prefix)), #default_value)?,
				});
			} else {
				fields_tokens.extend(quote! {
					#field_name: service_skeleton::config::determine_value(&format!(#env_var_format_string, prefix), #value_parser, map.get(&format!(#env_var_format_string, prefix)), #default_value)?,
				});
			}
		}

		tokens.extend(quote! {
			impl #imp ServiceConfig for #struct_name #ty #wher {
				fn from_env_vars(prefix: &str, vars: impl Iterator<Item = (String, String)>) -> Result<#struct_name, service_skeleton::Error> {
					let prefix = service_skeleton::heck::AsShoutySnekCase(prefix).to_string();
					let map: std::collections::HashMap<String, String> = vars.collect();

					#sensitive_env_vars_tokens

					Ok(#struct_name {
						#fields_tokens
					})
				}
			}
		});
	}
}

// Determine if the given type is an Option<T>, and if so, return Some(T), otherwise return None.
//
fn optionalise_type(ty: &syn::Type) -> Option<&syn::Type> {
	#[allow(clippy::wildcard_enum_match_arm)] // Yes, that's rather the point here
	match ty {
		syn::Type::Path(tp) if tp.qself.is_none() => {
			let path_idents = tp.path.segments.iter().fold(String::new(), |mut s, v| {
				s.push_str(&v.ident.to_string());
				s.push_str("->");
				s
			});
			if vec![
				"Option->",
				"std->option->Option->",
				"core->option->Option->",
			]
			.into_iter()
			.any(|s| *s == path_idents)
			{
				#[allow(clippy::unwrap_used)] // There has to be segments if we got here
				if let syn::PathArguments::AngleBracketed(args) =
					&tp.path.segments.iter().last().unwrap().arguments
				{
					if let Some(syn::GenericArgument::Type(t)) = &args.args.iter().next() {
						Some(t)
					} else {
						None
					}
				} else {
					None
				}
			} else {
				None
			}
		}
		_ => None,
	}
}

#[derive(Debug, FromField)]
#[darling(attributes(config))]
struct ServiceConfigField {
	ident: Option<syn::Ident>,
	ty: syn::Type,

	default_value: Option<SpannedValue<String>>,
	value_parser: Option<SpannedValue<ExprPath>>,
	sensitive: Flag,
}

impl ServiceConfigField {
	fn is_sensitive(&self) -> bool {
		self.sensitive.is_present()
	}
}

#[proc_macro_derive(ServiceConfig, attributes(config))]
pub fn derive_service_config(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input);

	let receiver = match ServiceConfigReceiver::from_derive_input(&input) {
		Ok(r) => r,
		Err(e) => return e.write_errors().into(),
	};

	quote!(#receiver).into()
}

// Only used in integration tests
#[cfg(test)]
use trybuild as _;
