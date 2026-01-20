//! Proc macros for teremock crate

#![allow(clippy::match_single_binding)]
#![allow(clippy::to_string_in_format_args)]

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Data, DeriveInput, Fields, PathArguments, Type, TypeGroup};

#[proc_macro_derive(Changeable)]
pub fn changeable_derive(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    // Used in the quasi-quotation below as `#name`
    let name = input.ident;

    // Generate an iterator over the fields
    let methods = if let Data::Struct(ref data) = input.data {
        match data.fields {
            Fields::Named(ref fields) => {
                fields.named.iter().map(|f| {
                    let field_name = &f.ident;
                    let field_type = &f.ty;
                    let field_visibility = &f.vis;
                    // If field is private, return
                    match field_visibility {
                        syn::Visibility::Public(_) => (),
                        _ => return quote! {},
                    }

                    // Because of regular macros, some of the types can be in a group
                    let type_path = match field_type {
                        syn::Type::Path(type_path) => type_path,
                        syn::Type::Group(ref type_group) => match type_group {
                            TypeGroup {
                                group_token: _,
                                ref elem,
                            } => {
                                if let syn::Type::Path(ref type_path) = **elem {
                                    type_path
                                } else {
                                    panic!("Unsupported field type")
                                }
                            }
                        },
                        _ => panic!("Unsupported field type"),
                    };

                    let last_segment = type_path.path.segments.last().unwrap();
                    if last_segment.ident == "Option" {
                        // Idk wtf this does, but somehow i managed to make it work
                        let inner_type = if let syn::PathArguments::AngleBracketed(args) =
                            &last_segment.arguments
                        {
                            if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first()
                            {
                                inner_type
                            } else {
                                panic!("Unsupported Option field type")
                            }
                        } else {
                            panic!("Unsupported Option field type")
                        };

                        let doc_comment = format!("Sets the {field_name} value of the {struct_name} to value, converting it to needed Option type.", struct_name = name.to_string(), field_name = field_name.clone().unwrap().to_string());
                        quote! {
                            #[doc = #doc_comment]
                            pub fn #field_name<T: Into<#inner_type>>(mut self, value: T) -> Self {
                                self.#field_name = Some(value.into());
                                self
                            }
                        }
                    // Next is just a bunch of useful conversions, like &str to String, i64 to ChatId etc.
                    } else if last_segment.ident == "String" {
                        let doc_comment = format!("Sets the {field_name} value of the {struct_name} to value, converting it to String.

# Example
```
use teremock::{struct_name};
let builder = {struct_name}::new().{field_name}(\"test\");
assert_eq!(builder.{field_name}, \"test\".to_string());
```
", struct_name = name.to_string(), field_name = field_name.clone().unwrap().to_string());
                        quote! {
                            #[doc = #doc_comment]
                            pub fn #field_name<T: Into<String>>(mut self, value: T) -> Self {
                                self.#field_name = value.into();
                                self
                            }
                        }
                    } else if last_segment.ident == "ChatId" {
                        let doc_comment = format!("Sets the {field_name} value of the {struct_name} to value, converting it to ChatId.

Accepts any type that implements `IntoChatId`, including:
- `i64` - raw chat ID value
- `ChatId` - directly pass a ChatId
- `UserId` - for private chats where chat ID equals user ID

# Example
```
use teremock::{struct_name};
let builder = {struct_name}::new().{field_name}(1234);
assert_eq!(builder.{field_name}, teloxide::types::ChatId(1234));
```
", field_name = field_name.clone().unwrap().to_string(), struct_name = name.to_string());
                        quote! {
                            #[doc = #doc_comment]
                            pub fn #field_name(mut self, value: impl crate::IntoChatId) -> Self {
                                self.#field_name = value.into_chat_id();
                                self
                            }
                        }
                    } else if last_segment.ident == "UserId" {
                        let doc_comment = format!("Sets the {field_name} value of the {struct_name} to value, converting it to UserId.

Accepts any type that implements `IntoUserId`, including:
- `u64` - raw user ID value
- `UserId` - directly pass a UserId

# Example
```
use teremock::{struct_name};
let builder = {struct_name}::new().{field_name}(1234);
assert_eq!(builder.{field_name}, teloxide::types::UserId(1234));
```
", field_name = field_name.clone().unwrap().to_string(), struct_name = name.to_string());
                        quote! {
                            #[doc = #doc_comment]
                            pub fn #field_name(mut self, value: impl crate::IntoUserId) -> Self {
                                self.#field_name = value.into_user_id();
                                self
                            }
                        }
                    } else if last_segment.ident == "MessageId" {
                        let doc_comment = format!("Sets the {field_name} value of the {struct_name} to value, converting it to MessageId.

Accepts any type that implements `IntoMessageId`, including:
- `i32` - raw message ID value
- `MessageId` - directly pass a MessageId

# Example
```
use teremock::{struct_name};
let builder = {struct_name}::new().{field_name}(1234);
assert_eq!(builder.{field_name}, teloxide::types::MessageId(1234));
```
", field_name = field_name.clone().unwrap().to_string(), struct_name = name.to_string());
                        quote! {
                            #[doc = #doc_comment]
                            pub fn #field_name(mut self, value: impl crate::IntoMessageId) -> Self {
                                self.#field_name = value.into_message_id();
                                self
                            }
                        }
                    } else {
                        let doc_comment = format!("Sets the {field_name} value of the {struct_name} to value, converting it via Into trait.", struct_name = name.to_string(), field_name = field_name.clone().unwrap().to_string());
                        quote! {
                            #[doc = #doc_comment]
                            pub fn #field_name(mut self, value: impl Into<#field_type>) -> Self {
                                self.#field_name = value.into();
                                self
                            }
                        }
                    }
                })
            }
            _ => panic!("Changeable macro only works on structs with named fields"),
        }
    } else {
        panic!("Changeable macro only works on structs");
    };

    // Build the output
    let expanded = quote! {
        impl #name {
            #(#methods)*
        }
    };

    // Hand the output tokens back to the compiler
    TokenStream::from(expanded)
}

#[proc_macro_derive(SerializeRawFields)]
pub fn serialize_raw_fields_derive(input: TokenStream) -> TokenStream {
    // This proc macro just creates a body struct out of the raw request fields
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident.clone();

    let fields = if let Data::Struct(data_struct) = input.data {
        data_struct.fields
    } else {
        unimplemented!();
    };

    let field_serializers = fields.iter().filter(|field| field.ident.as_ref().unwrap() != "file_name" && field.ident.as_ref().unwrap() != "file_data").map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;

        // Check if the field type is Option<T>
        let is_option = if let Type::Path(type_path) = field_type {
            if let Some(segment) = type_path.path.segments.last() {
                if segment.ident == "Option" {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        args.args.len() == 1
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        let key = field_name.to_string();

        if field_type.clone().to_token_stream().to_string() == syn::parse_str::<syn::Type>("Option<String>").unwrap().to_token_stream().to_string() {
            quote! {
                #field_name: fields.get(#key).cloned(),
            }
        } else if field_type.clone().to_token_stream().to_string() == syn::parse_str::<syn::Type>("String").unwrap().to_token_stream().to_string() {
            quote! {
                #field_name: fields.get(#key)?.to_string(),
            }
        } else if !is_option {
            quote! {
                #field_name: serde_json::from_str(&fields.get(#key).unwrap_or(&String::new())).ok()?,
            }
        } else {
            quote! {
                #field_name: serde_json::from_str(&fields.get(#key).unwrap_or(&String::new())).ok(),
            }
        }
    });

    let expanded = quote! {
        impl SerializeRawFields for #name {
            fn serialize_raw_fields(
                fields: &HashMap<String, String>,
                attachments: &HashMap<String, Attachment>,
                file_type: FileType,
            ) -> Option<Self> {
                let attachment = attachments.keys().last();
                let (file_name, file_data) = match attachment {
                    Some(attachment) => {
                        let attach = attachments.get_key_value(attachment)?;
                        (attach.1.file_name.clone(), &attach.1.file_data)
                    },
                    None => match file_type {
                        FileType::Photo => ("no_name.jpg".to_string(), fields.get("photo")?),
                        FileType::Video => ("no_name.mp4".to_string(), fields.get("video")?),
                        FileType::Audio => ("no_name.mp3".to_string(), fields.get("audio")?),
                        FileType::Document => ("no_name.txt".to_string(), fields.get("document")?),
                        FileType::Sticker => ("no_name.png".to_string(), fields.get("sticker")?),
                        FileType::Voice => ("no_name.mp3".to_string(), fields.get("voice")?),
                        FileType::VideoNote => ("no_name.mp4".to_string(), fields.get("video_note")?),
                        FileType::Animation => ("no_name.gif".to_string(), fields.get("animation")?),
                    },
                };

                Some(#name {
                    file_name: file_name.to_string(),
                    file_data: file_data.to_string(),
                    #(#field_serializers)*
                })
            }
        }
    };

    TokenStream::from(expanded)
}
