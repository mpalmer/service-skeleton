use darling::{ast, util::Flag, util::SpannedValue, FromDeriveInput, FromField};
use heck::AsShoutySnekCase;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{parse_macro_input, spanned::Spanned, ExprPath, Ident, Type};

#[proc_macro_derive(ServiceConfig, attributes(config))]
pub fn derive_service_config(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input);

	let receiver = match ServiceConfigReceiver::from_derive_input(&input) {
		Ok(r) => r,
		Err(e) => return e.write_errors().into(),
	};

	quote!(#receiver).into()
}

#[derive(Debug, FromDeriveInput)]
#[darling(supports(struct_named))]
struct ServiceConfigReceiver {
	ident: Ident,
	generics: syn::Generics,
	data: ast::Data<(), SpannedValue<ServiceConfigField>>,
}

impl ToTokens for ServiceConfigReceiver {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let struct_name = &self.ident;
		let (imp, ty, wher) = self.generics.split_for_impl();

		let mut fields: Vec<TokenStream> = Vec::new();
		let mut purges: Vec<TokenStream> = Vec::new();

		#[allow(clippy::expect_used)] // Ensured by darling(supports(struct_named))
		for f in self
			.data
			.as_ref()
			.take_struct()
			.expect("data to be a struct")
			.fields
		{
			fields.push(f.field_init());

			purges.push(f.purge_sensitive());
		}

		tokens.extend(quote! {
			impl #imp ServiceConfig for #struct_name #ty #wher {
				fn from_env_vars(prefix: &str, vars: impl Iterator<Item = (String, String)>) -> Result<#struct_name, service_skeleton::Error> {
					let prefix = ::service_skeleton::heck::AsShoutySnekCase(prefix).to_string();
					let var_map: ::std::collections::HashMap<String, String> = vars.collect();
					let mut key_map: ::std::collections::HashMap<::service_skeleton::config::Key, ::secrecy::SecretString> = ::std::collections::HashMap::new();

					let cfg = #struct_name {
						#(#fields)*
					};

					#(#purges)*

					Ok(cfg)
				}
			}
		});
	}
}

#[derive(Debug, FromField)]
#[darling(attributes(config))]
struct ServiceConfigField {
	ident: Option<Ident>,
	ty: Type,

	default_value: Option<SpannedValue<String>>,
	value_parser: Option<SpannedValue<ExprPath>>,
	encrypted: Flag,
	sensitive: Flag,
	key_file_field: Option<SpannedValue<String>>,
}

impl ServiceConfigField {
	fn field_init(&self) -> TokenStream {
		let field_name = self.field_name();
		let fmt_str = Self::env_var_format_string(&field_name.to_string());
		let value_parser = self.value_parser();
		let default_value = self.default_value();
		let fetch_value = self.fetch_value();

		if self.is_optional() {
			quote_spanned! { self.ident.span()=>
				#field_name: ::service_skeleton::config::determine_optional_value(
					&format!(#fmt_str, prefix),
					#value_parser,
					#fetch_value,
					#default_value
				)?,
			}
		} else {
			quote_spanned! { self.ident.span()=>
				#field_name: ::service_skeleton::config::determine_value(
					&format!(#fmt_str, prefix),
					#value_parser,
					#fetch_value,
					#default_value
				)?,
			}
		}
	}

	fn fetch_value(&self) -> TokenStream {
		let field_var_fmt_str = Self::env_var_format_string(&self.field_name().to_string());

		if self.encrypted.is_present() {
			if let Some(ref key_file_field) = self.key_file_field {
				let key_var_fmt_str = Self::env_var_format_string(key_file_field);

				quote_spanned! { self.ident.span()=>
					::service_skeleton::config::fetch_encrypted_field(&var_map, &mut key_map, &format!(#field_var_fmt_str, prefix), &::service_skeleton::config::Key::File(format!(#key_var_fmt_str, prefix)))?.as_deref()
				}
			} else {
				quote_spanned! { self.encrypted.span()=>
					compile_error!("field is encrypted but no key_file was specified to decrypt");
				}
			}
		} else {
			quote_spanned! { self.ident.span()=>
				var_map.get(&format!(#field_var_fmt_str, prefix)).map(::std::string::String::as_str)
			}
		}
	}

	fn purge_sensitive(&self) -> TokenStream {
		if self.is_sensitive() {
			let fmt_str = Self::env_var_format_string(&self.field_name().to_string());
			quote_spanned! { self.ident.span()=>
				::tracing::debug!("Removing sensitive env var {}", format!(#fmt_str, prefix));
				::std::env::remove_var(&format!(#fmt_str, prefix));
			}
		} else {
			quote! {}
		}
	}

	fn field_name(&self) -> &Ident {
		#[allow(clippy::expect_used)]
		self.ident
			.as_ref()
			.expect("named field does not have a field")
	}

	fn env_var_format_string(field_name: &str) -> String {
		format!(
			"{{}}_{shouty_field_name}",
			shouty_field_name = AsShoutySnekCase(field_name)
		)
	}

	fn is_sensitive(&self) -> bool {
		self.sensitive.is_present()
	}

	fn is_optional(&self) -> bool {
		#[allow(clippy::wildcard_enum_match_arm)] // Yes, that's rather the point here
		match &self.ty {
			Type::Path(tp) if tp.qself.is_none() => {
				let path_idents = tp.path.segments.iter().fold(String::new(), |mut s, v| {
					s.push_str(&v.ident.to_string());
					s.push_str("->");
					s
				});
				vec![
					"Option->",
					"std->option->Option->",
					"core->option->Option->",
				]
				.into_iter()
				.any(|s| *s == path_idents)
			}
			_ => false,
		}
	}

	/// Determine the type of the field that we will want to parse into -- essentially,
	/// the field's specified type less any wrapping Option<>, if present.
	fn value_type(&self) -> &Type {
		#[allow(clippy::wildcard_enum_match_arm)] // Yes, that's rather the point here
		match &self.ty {
			Type::Path(tp) if tp.qself.is_none() => {
				if self.is_optional() {
					#[allow(clippy::unwrap_used)] // There has to be segments if we got here
					if let syn::PathArguments::AngleBracketed(args) =
						&tp.path.segments.iter().next_back().unwrap().arguments
					{
						if let Some(syn::GenericArgument::Type(t)) = &args.args.iter().next() {
							t
						} else {
							&self.ty
						}
					} else {
						&self.ty
					}
				} else {
					&self.ty
				}
			}
			_ => &self.ty,
		}
	}

	fn value_parser(&self) -> TokenStream {
		if let Some(value_parser) = &self.value_parser {
			// The as_ref() turns SpannedValue<T> into something that impls ToTokens, somehow
			let parser = value_parser.as_ref();
			quote_spanned! { value_parser.span()=> #parser }
		} else {
			let value_type = self.value_type();
			quote_spanned! { self.ident.span()=>
				|s: &str| s.parse::<#value_type>()
			}
		}
	}

	fn default_value(&self) -> TokenStream {
		if let Some(default_value) = &self.default_value {
			// The as_ref() turns SpannedValue<T> into something that impls ToTokens, somehow
			let default_value = default_value.as_ref();
			quote_spanned! { default_value.span()=> Some(#default_value) }
		} else {
			quote_spanned! { self.ident.span()=> None }
		}
	}
}

// Only used in integration tests
#[cfg(test)]
use trybuild as _;
