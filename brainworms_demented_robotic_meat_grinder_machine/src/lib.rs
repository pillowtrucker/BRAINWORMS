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
#[proc_macro_derive(Scenic, attributes(user_data_struct))]
pub fn derive_scenic_partial(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident;
    let the_user_data_struct: &Ident = &input
        .attrs
        .iter()
        .find(|a| {
            a.path()
                .get_ident()
                .is_some_and(|aa| aa == "user_data_struct")
        })
        .map(|aa| {
            aa.parse_args()
                .expect("user_data_struct attribute required")
        })
        .expect("user_data_struct attribute required");
    let tokens = quote! {
        impl brainworms_lib::theater::play::scene::Scenic<#the_user_data_struct> for #ident {

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
                               renderer: std::sync::Arc<brainworms_lib::rend3::Renderer>,
                               routines: std::sync::Arc<brainworms_lib::theater::play::backstage::plumbing::DefaultRoutines>,
                               rts: &brainworms_lib::tokio::runtime::Runtime,
                               orchestra: std::sync::Arc<brainworms_lib::theater::play::orchestra::Orchestra>,
                               user_data: std::sync::Arc<brainworms_lib::parking_lot::Mutex<#the_user_data_struct>>
            ) {
                self.implement(
                    settings,
                    event_loop,
                    renderer,
                    routines,
                    rts,
                    orchestra,
                    user_data)
            }
            fn scene_starting_cam_info(&self) -> brainworms_lib::theater::play::scene::CamInfo {
                self.starting_cam_info()
            }

        }
    };

    tokens.into()
}
#[proc_macro_derive(Choral, attributes(user_data_struct))]
pub fn derive_choral(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident;
    let the_user_data_struct: &Ident = &input
        .attrs
        .iter()
        .find(|a| {
            a.path()
                .get_ident()
                .is_some_and(|aa| aa == "user_data_struct")
        })
        .map(|aa| {
            aa.parse_args()
                .expect("user_data_struct attribute required")
        })
        .expect("user_data_struct attribute required");
    let tokens = quote! {
        impl brainworms_lib::theater::play::scene::chorus::Choral<#the_user_data_struct> for #ident {
            fn implement_chorus_for_choral(&self, egui_ctx: brainworms_lib::egui::Context, orchestra: std::sync::Arc<brainworms_lib::theater::play::orchestra::Orchestra>,settings: &brainworms_lib::theater::basement::cla::GameProgrammeSettings, user_data: std::sync::Arc<brainworms_lib::parking_lot::Mutex<#the_user_data_struct>>) {
                self.implement_chorus(egui_ctx, orchestra,settings,user_data);
            }
            fn chorus_uuid(&self) -> brainworms_lib::uuid::Uuid {
                self.uuid
            }
            fn chorus_name(&self) -> &str {
                &self.name
            }
            fn chorus_definition(&mut self) -> &mut brainworms_lib::theater::play::Definitions {
                &mut self.definition
            }
            fn chorus_implementation(&mut self) -> &mut Option<brainworms_lib::theater::play::Implementations> {
                &mut self.implementation
            }
            fn define_chorus(&mut self) {
                self.define()
            }

        }
    };

    tokens.into()
}

// enum_dispatch doesn't work across crates..
#[proc_macro_derive(Playable, attributes(input_context_enum, user_data_struct))]
pub fn derive_playable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = &input.ident;
    //    let mut tokens: TokenStream;
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
    let the_user_data_struct: &Ident = &input
        .attrs
        .iter()
        .find(|a| {
            a.path()
                .get_ident()
                .is_some_and(|aa| aa == "user_data_struct")
        })
        .map(|aa| {
            aa.parse_args()
                .expect("user_data_struct attribute required")
        })
        .expect("user_data_struct attribute required");
    let out = match input.data {
        syn::Data::Struct(_) => {
            quote! {
            use brainworms_lib::theater::play::scene::chorus::Choral as _;
            impl brainworms_lib::theater::play::Playable<#the_input_context_enum, #the_user_data_struct> for #ident
            {
                fn playable_uuid(&self) -> brainworms_lib::uuid::Uuid {
                    self.chorus_uuid()
                }

                fn playable_name(&self) ->  &str {
                    self.chorus_name()
                }

                fn playable_definition(&mut self) ->  &mut brainworms_lib::theater::play::Definitions {
                    self.chorus_definition()
                }

                fn playable_implementation(&mut self) ->  &mut Option<brainworms_lib::theater::play::Implementations> {
                    self.chorus_implementation()
                }

                fn starting_cam_info(&self) -> brainworms_lib::theater::play::scene::CamInfo {
                    Default::default()
                }

                fn implement_playable(&mut self,
                                      settings: &brainworms_lib::theater::basement::cla::GameProgrammeSettings,
                                      event_loop: &brainworms_lib::winit::event_loop::EventLoop<brainworms_lib::MyEvent>,
                                      renderer:std::sync::Arc<brainworms_lib::rend3::Renderer>,
                                      routines:std::sync::Arc<brainworms_lib::theater::play::backstage::plumbing::DefaultRoutines>,
                                      rts: &brainworms_lib::tokio::runtime::Runtime,
                                      orchestra:std::sync::Arc<brainworms_lib::theater::play::orchestra::Orchestra>,
                                      user_data: std::sync::Arc<brainworms_lib::parking_lot::Mutex<#the_user_data_struct>>
                ) {

                }

                fn define_playable(&mut self) {
                    self.define_chorus()
                }

                fn implement_chorus_for_playable(&self,egui_ctx:brainworms_lib::egui::Context,orchestra:std::sync::Arc<brainworms_lib::theater::play::orchestra::Orchestra>,      settings: &brainworms_lib::theater::basement::cla::GameProgrammeSettings,                                user_data: std::sync::Arc<brainworms_lib::parking_lot::Mutex<#the_user_data_struct>>) {
                    self.implement_chorus_for_choral(egui_ctx,orchestra,settings,user_data)
                }

                fn handle_input_for_playable(&mut self,settings: &brainworms_lib::theater::basement::cla::GameProgrammeSettings,state: &mut brainworms_lib::GameProgrammeState<#the_input_context_enum>,window: &std::sync::Arc<brainworms_lib::winit::window::Window>,) {
                    // egui has its own input handling
                }
            }

                        }
        }
        syn::Data::Enum(enum_data) => {
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
                        #ident :: #name(inner) => inner.#fn_name(#(#fn_args),*)
                    }
                })
            };
            let imp_pl_uuid = imp_fn("playable_uuid", "");
            let imp_pl_name = imp_fn("playable_name", "");
            let imp_pl_start_cam = imp_fn("starting_cam_info", "");
            let def_pl = imp_fn("define_playable", "");
            let imp_pl = imp_fn(
                "implement_playable",
                "settings,event_loop,renderer,routines,rts,orchestra,user_data",
            );
            let imp_chr = imp_fn(
                "implement_chorus_for_playable",
                "egui_ctx,orchestra,settings,user_data",
            );
            let pl_def = imp_fn("playable_definition", "");
            let pl_imp = imp_fn("playable_implementation", "");
            let pl_inp = imp_fn("handle_input_for_playable", "settings,state,window");
            quote! {
            impl brainworms_lib::theater::play::Playable<#the_input_context_enum, #the_user_data_struct> for #ident {
                fn playable_uuid(&self) -> brainworms_lib::uuid::Uuid {
                    match self {
                        #(#imp_pl_uuid),*
                    }
                }
                fn playable_name(&self) -> &str {
                    match self {
                        #(#imp_pl_name),*
                    }
                }
                fn starting_cam_info(&self) -> brainworms_lib::theater::play::scene::CamInfo {
                    match self {
                        #(#imp_pl_start_cam),*
                    }
                }
                fn implement_playable(&mut self,settings: &brainworms_lib::theater::basement::cla::GameProgrammeSettings,
                                      event_loop: &brainworms_lib::winit::event_loop::EventLoop<brainworms_lib::MyEvent>,
                                      renderer:std::sync::Arc<brainworms_lib::rend3::Renderer>,
                                      routines:std::sync::Arc<brainworms_lib::theater::play::backstage::plumbing::DefaultRoutines>,
                                      rts: &brainworms_lib::tokio::runtime::Runtime,
                                      orchestra:std::sync::Arc<brainworms_lib::theater::play::orchestra::Orchestra>,
                                      user_data: std::sync::Arc<brainworms_lib::parking_lot::Mutex<#the_user_data_struct>>
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
                fn implement_chorus_for_playable(&self, egui_ctx: brainworms_lib::egui::Context, orchestra: std::sync::Arc<brainworms_lib::theater::play::orchestra::Orchestra>,settings: &brainworms_lib::theater::basement::cla::GameProgrammeSettings,user_data: std::sync::Arc<brainworms_lib::parking_lot::Mutex<#the_user_data_struct>>) {
                    match self {
                        #(#imp_chr),*
                    }
                }
                fn playable_definition(&mut self) -> &mut brainworms_lib::theater::play::Definitions {
                    match self {
                        #(#pl_def),*
                    }
                }
                fn playable_implementation(&mut self) -> &mut Option<brainworms_lib::theater::play::Implementations> {
                    match self {
                        #(#pl_imp),*
                    }
                }
                fn handle_input_for_playable(&mut self,settings: &brainworms_lib::theater::basement::cla::GameProgrammeSettings,state: &mut brainworms_lib::GameProgrammeState<#the_input_context_enum>,window: &std::sync::Arc<brainworms_lib::winit::window::Window>) {
                    match self {
                        #(#pl_inp),*
                    }
                }
            }}
        }
        syn::Data::Union(_) => {
            panic!("but we're just like a family in here. There's a fussball table and 5 kinds of sweetened cereal");
        }
    };
    //    println!("{}", out);
    out.into()
}
