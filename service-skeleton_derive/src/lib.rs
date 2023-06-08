use darling::{ast, util::SpannedValue, FromDeriveInput, FromField};
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

			let value_parser = if let Some(value_parser) = &f.value_parser {
				let parser = value_parser.as_ref();
				quote_spanned! { value_parser.span()=> #parser }
			} else {
				quote_spanned! { f.span()=>
					|s: &str| s.parse::<#field_type>()
				}
			};

			let default_value = if let Some(default_value) = &f.default_value {
				let value = default_value.as_ref();
				quote_spanned! { default_value.span()=> Some(#value) }
			} else {
				quote_spanned! { f.span()=> None }
			};

			fields_tokens.extend(quote! {
				#field_name: service_skeleton::config::determine_value(#field_name_as_string, #value_parser, map.get(&format!(#env_var_format_string, prefix)), #default_value)?,
			});
		}

		tokens.extend(quote! {
			impl #imp ServiceConfig for #struct_name #ty #wher {
				fn from_env_vars(prefix: &str, vars: impl Iterator<Item = (String, String)>) -> Result<#struct_name, service_skeleton::Error> {
					let prefix = service_skeleton::heck::AsShoutySnekCase(prefix).to_string();
					let map: std::collections::HashMap<String, String> = vars.collect();

					Ok(#struct_name {
						#fields_tokens
					})
				}
			}
		});
	}
}

#[derive(Debug, FromField)]
#[darling(attributes(config))]
struct ServiceConfigField {
	ident: Option<syn::Ident>,
	ty: syn::Type,

	default_value: Option<SpannedValue<String>>,
	value_parser: Option<SpannedValue<ExprPath>>,
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
