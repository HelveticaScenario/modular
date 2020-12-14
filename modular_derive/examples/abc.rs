use proc_macro2::{TokenStream, TokenTree};
use syn::{Attribute, DeriveInput};

pub fn main() {
    let derive_input: DeriveInput = syn::parse_str(
        r#"
        #[test_attr("baz")]
        struct ABC;
    "#,
    )
    .unwrap();
    // println!("{:?}", derive_input);
    println!(
        "{:?}",
        derive_input
            .attrs
    );
    // TokenStream [Group { delimiter: Parenthesis, stream: TokenStream [Ident { sym: name }, Punct { char: '=', spacing: Alone }, Literal { lit: "foo" }, Punct { char: ',', spacing: Alone }, Ident { sym: description }, Punct { char: '=', spacing: Alone }, Literal { lit: "baz" }] }]
    // Attribute { pound_token: Pound, style: Outer, bracket_token: Bracket, path: Path { leading_colon: None, segments: [PathSegment { ident: Ident(test_attr), arguments: None }] }, tokens: TokenStream [Group { delimiter: Parenthesis, stream: TokenStream [Ident { sym: name }, Punct { char: '=', spacing: Alone }, Literal { lit: "foo" }, Punct { char: ',', spacing: Alone }, Ident { sym: description }, Punct { char: '=', spacing: Alone }, Literal { lit: "baz" }] }] }
    // [Attribute { pound_token: Pound, style: Outer, bracket_token: Bracket, path: Path { leading_colon: None, segments: [PathSegment { ident: Ident(test_attr), arguments: None }] }, tokens: TokenStream [Group { delimiter: Parenthesis, stream: TokenStream [Ident { sym: name }, Punct { char: '=', spacing: Alone }, Literal { lit: "foo" }, Punct { char: ',', spacing: Alone }, Ident { sym: description }, Punct { char: '=', spacing: Alone }, Literal { lit: "baz" }] }] }]
    // DeriveInput { attrs: [Attribute { pound_token: Pound, style: Outer, bracket_token: Bracket, path: Path { leading_colon: None, segments: [PathSegment { ident: Ident(test_attr), arguments: None }] }, tokens: TokenStream [Group { delimiter: Parenthesis, stream: TokenStream [Ident { sym: name }, Punct { char: '=', spacing: Alone }, Literal { lit: "foo" }, Punct { char: ',', spacing: Alone }, Ident { sym: description }, Punct { char: '=', spacing: Alone }, Literal { lit: "baz" }] }] }], vis: Inherited, ident: Ident(ABC), generics: Generics { lt_token: None, params: [], gt_token: None, where_clause: None }, data: Struct(DataStruct { struct_token: Struct, fields: Unit, semi_token: Some(Semi) }) }

    // let a = DeriveInput {
    //     attrs: [],
    //     vis: Inherited,
    //     ident: Ident(Abc),
    //     generics: Generics {
    //         lt_token: None,
    //         params: [],
    //         gt_token: None,
    //         where_clause: None,
    //     },
    //     data: Struct(DataStruct {
    //         struct_token: Struct,
    //         fields: Named(FieldsNamed {
    //             brace_token: Brace,
    //             named: [Field {
    //                 attrs: [Attribute {
    //                     pound_token: Pound,
    //                     style: Outer,
    //                     bracket_token: Bracket,
    //                     path: Path {
    //                         leading_colon: None,
    //                         segments: [PathSegment {
    //                             ident: Ident(attr_test),
    //                             arguments: None,
    //                         }],
    //                     },
    //                     tokens_TokenStream: [Group {
    //                         delimiter: Parenthesis,
    //                         stream_TokenStream: [
    //                             Ident { sym: name },
    //                             Punct {
    //                                 char: '=',
    //                                 spacing: Alone,
    //                             },
    //                             Literal { lit: "foo" },
    //                             Punct {
    //                                 char: ',',
    //                                 spacing: Alone,
    //                             },
    //                             Ident { sym: description },
    //                             Punct {
    //                                 char: '=',
    //                                 spacing: Alone,
    //                             },
    //                             Literal { lit: "baz" },
    //                         ],
    //                     }],
    //                 }],
    //                 vis: Inherited,
    //                 ident: Some(Ident(bar)),
    //                 colon_token: Some(Colon),
    //                 ty: Path(TypePath {
    //                     qself: None,
    //                     path: Path {
    //                         leading_colon: None,
    //                         segments: [PathSegment {
    //                             ident: Ident(String),
    //                             arguments: None,
    //                         }],
    //                     },
    //                 }),
    //             }],
    //         }),
    //         semi_token: None,
    //     }),
    // };
    // let a = TokenStream [
    //     Group {
    //         delimiter: Parenthesis,
    //         stream: [
    //             Ident {
    //                 sym: name
    //             },
    //             Punct {
    //                 char: '=',
    //                 spacing: Alone
    //             },
    //             Literal {
    //                 lit: "foo"
    //             },
    //             Punct {
    //                 char: ',',
    //                 spacing: Alone
    //             },
    //             Ident {
    //                 sym: description
    //             },
    //             Punct {
    //                 char: '=',
    //                 spacing: Alone
    //             },
    //             Literal {
    //                 lit: "baz"
    //             }
    //         ]
    //     }
    // ];
}
