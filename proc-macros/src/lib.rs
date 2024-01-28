use proc_macro::TokenStream;

use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Example of [function-like procedural macro][1].
///
/// [1]: https://doc.rust-lang.org/reference/procedural-macros.html#function-like-procedural-macros
#[proc_macro]
pub fn my_macro(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let tokens = quote! {
        #input

        struct Hello;
    };

    tokens.into()
}

/// Example of user-defined [derive mode macro][1]
///
/// [1]: https://doc.rust-lang.org/reference/procedural-macros.html#derive-mode-macros
#[proc_macro_derive(Scenic)]
pub fn derive_scenic_partial(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident;
    let tokens = quote! {

        impl Scenic for #ident {
            /*
            fn scene_definition(&mut self) -> &mut SceneDefinition {
                let Definitions::SceneDefinition(definition) = &mut self.definition else {
                    panic!("scene has non-scene definition")
                };
                definition
            }
            fn scene_implementation(&mut self) -> &mut Option<SceneImplementation> {
                let Implementations::SceneImplementation(implementation) = &mut self.implementation else {
                    panic!("scene has non-scene implementation")
                };
                implementation
            }
            */
            fn raw_definition(&mut self) -> &mut brainworms_lib::theater::play::Definitions {
                &mut self.definition
            }
            fn raw_implementation(&mut self) -> &mut Option<brainworms_lib::theater::play::Implementations> {
                &mut self.implementation
            }
            fn scene_uuid(&self) -> brainworms_lib::uuid::Uuid {
                self.uuid
            }
            fn scene_name(&self) -> &str {
                &self.name
            }
            fn define_scene(&mut self) {
                self.define()
            }
            fn implement_scene(&mut self,
                               settings: &brainworms_lib::theater::basement::cla::GameProgrammeSettings,
                               event_loop: &brainworms_lib::winit::event_loop::EventLoop<brainworms_lib::MyEvent>,
                               renderer: Arc<brainworms_lib::rend3::Renderer>,
                               routines: Arc<brainworms_lib::theater::play::backstage::plumbing::DefaultRoutines>,
                               rts: &brainworms_lib::tokio::runtime::Runtime,
                               orchestra: Arc<brainworms_lib::theater::play::orchestra::Orchestra>
            ) {
                self.implement(
                    settings,
                    event_loop,
                    renderer,
                    routines,
                    rts,
                    orchestra)
            }
            fn scene_starting_cam_info(&self) -> brainworms_lib::theater::play::scene::CamInfo {
                self.starting_cam_info()
            }

        }
    };

    tokens.into()
}
#[proc_macro_derive(Choral)]
pub fn derive_choral_partial(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident;
    let tokens = quote! {
        impl Choral for #ident {
            fn implement_chorus_for_choral(&self, egui_ctx: brainworms_lib::egui::Context) {
                self.implement_chorus(egui_ctx);
            }

        }
    };

    tokens.into()
}

// enum_dispatch doesn't work across crates..
#[proc_macro_derive(Playable, attributes(input_context_enum))]
pub fn derive_playable_for_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let enum_ident = &input.ident;
    //    let mut tokens: TokenStream;
    let out = match input.data {
        syn::Data::Struct(_) => {
            panic!("enums only");
        }
        syn::Data::Enum(enum_data) => {
            let the_input_context_enum: &Ident = &input
                .attrs
                .iter()
                .find(|a| {
                    a.path()
                        .get_ident()
                        .is_some_and(|aa| aa == "input_context_enum")
                })
                .map(|aa| {
                    aa.parse_args()
                        .expect("input_context_enum attribute required")
                })
                .expect("input_context_enum attribute required");
            let variants = &enum_data.variants;
            let imp_fn = |fn_name, fn_args: &'static str| {
                variants.iter().map(move |v| {
                    let name = &v.ident;
                    let fn_name = Ident::new(fn_name, Span::call_site());
                    let fn_args = if fn_args.is_empty() {
                        vec![]
                    } else {
                        fn_args
                            .split(',')
                            .map(|a| Ident::new(a, Span::call_site()))
                            .collect()
                    };
                    quote! {
                        #enum_ident :: #name(inner) => inner.#fn_name(#(#fn_args),*)
                    }
                })
            };
            let imp_pl_uuid = imp_fn("playable_uuid", "");
            let imp_pl_name = imp_fn("playable_name", "");
            let imp_pl_start_cam = imp_fn("starting_cam_info", "");
            let def_pl = imp_fn("define_playable", "");
            let imp_pl = imp_fn(
                "implement_playable",
                "settings,event_loop,renderer,routines,rts,orchestra",
            );
            let imp_chr = imp_fn("implement_chorus_for_playable", "egui_ctx");
            let pl_def = imp_fn("playable_definition", "");
            let pl_imp = imp_fn("playable_implementation", "");
            let pl_inp = imp_fn("handle_input_for_playable", "settings,state,window");
            quote! {
            impl Playable<#the_input_context_enum> for #enum_ident {
                fn playable_uuid(&self) -> Uuid {
                    match self {
                        #(#imp_pl_uuid),*
                    }
                }
                fn playable_name(&self) -> &str {
                    match self {
                        #(#imp_pl_name),*
                    }
                }
                fn starting_cam_info(&self) -> CamInfo {
                    match self {
                        #(#imp_pl_start_cam),*
                    }
                }
                fn implement_playable(
                    &mut self,
                    settings: &GameProgrammeSettings,
                    event_loop: &EventLoop<MyEvent>,
                    renderer: Arc<Renderer>,
                    routines: Arc<DefaultRoutines>,
                    rts: &Runtime,
                    orchestra: Arc<Orchestra>
                ) {
                    match self {
                        #(#imp_pl),*
                    }
                }
                fn define_playable(&mut self) {
                    match self {
                        #(#def_pl),*
                    }
                }
                fn implement_chorus_for_playable(&self, egui_ctx: Context) {
                    match self {
                        #(#imp_chr),*
                    }
                }
                fn playable_definition(&mut self) -> &mut Definitions {
                    match self {
                        #(#pl_def),*
                    }
                }
                fn playable_implementation(&mut self) -> &mut Option<Implementations> {
                    match self {
                        #(#pl_imp),*
                    }
                }

                fn handle_input_for_playable(
                    &mut self,
                    settings: &GameProgrammeSettings,
                    state: &mut GameProgrammeState<#the_input_context_enum>,
                    window: &Arc<Window>,
                ) {
                    match self {
                        #(#pl_inp),*
                    }
                }
            }}
        }
        syn::Data::Union(_) => {
            panic!("enums only");
        }
    };
    //    println!("{}", out);
    out.into()
}

/*
// this is not worth the stupid RA errors
#[proc_macro_attribute]
pub fn add_common_playable_fields(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut item_struct = parse_macro_input!(input as ItemStruct);
    let _ = parse_macro_input!(args as syn::parse::Nothing);

    if let syn::Fields::Named(ref mut fields) = item_struct.fields {
        fields.named.push(
            syn::Field::parse_named
                .parse2(quote! { pub uuid: Uuid })
                .unwrap(),
        );
        fields.named.push(
            syn::Field::parse_named
                .parse2(quote! { pub name: String })
                .unwrap(),
        );
        fields.named.push(
            syn::Field::parse_named
                .parse2(quote! { pub definition: SceneDefinition })
                .unwrap(),
        );
        fields.named.push(
            syn::Field::parse_named
                .parse2(quote! { pub implementation: Option<SceneImplementation> })
                .unwrap(),
        );
    }

    quote! {
        #item_struct
    }
    .into()
}
*/
