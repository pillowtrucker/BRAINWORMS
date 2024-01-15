use proc_macro::TokenStream;

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
            fn raw_definition(&mut self) -> &mut Definitions {
                &mut self.definition
            }
            fn raw_implementation(&mut self) -> &mut Option<Implementations> {
                &mut self.implementation
            }
            fn scene_uuid(&self) -> Uuid {
                self.uuid
            }
            fn scene_name(&self) -> &str {
                &self.name
            }
            fn define_scene(&mut self) {
                self.define()
            }
            fn implement_scene(&mut self,
                               settings: &GameProgrammeSettings,
                               event_loop: &EventLoop<MyEvent>,
                               renderer: Arc<Renderer>,
                               routines: Arc<DefaultRoutines>,
                               rts: &Runtime,) {
                self.implement(
                    settings,
                    event_loop,
                    renderer,
                    routines,
                    rts)
            }
            fn scene_starting_cam_info(&self) -> CamInfo {
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
            fn implement_chorus_for_choral(&self, egui_ctx: Context) {
                self.implement_chorus(egui_ctx);
            }

        }
    };

    tokens.into()
}
/*
// enum_dispatch doesn't work across crates..
#[proc_macro_derive(EnumPlayable)]
pub fn derive_playable_for_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let enum_ident = input.ident;
    //    let mut tokens: TokenStream;
    match input.data {
        syn::Data::Struct(_) => panic!("enums only"),
        syn::Data::Enum(enum_data) => {
            for variant_ident in enum_data.variants.iter() {
                let tokens = quote! {
                    impl Playable for #variant_ident {
                        fn playable_uuid(&self) -> Uuid {
                            self.playable_uuid()
                        }

                        fn playable_name(&self) -> &str {
                            self.playable_name()
                        }

                        fn starting_cam_info(&self) -> CamInfo {
                            self.scene_starting_cam_info()
                        }

                        fn implement_playable(
                            &mut self,
                            settings: &GameProgrammeSettings,
                            event_loop: &EventLoop<MyEvent>,
                            renderer: Arc<Renderer>,
                            routines: Arc<DefaultRoutines>,
                            rts: &Runtime,
                        ) {
                            self.implement_scene(settings, event_loop, renderer, routines, rts)
                        }

                        fn define_playable(&mut self) {
                            self.define_scene()
                        }
                        fn implement_chorus_for_playable(&self, egui_ctx: Context) {
                            self.implement_chorus_for_choral(egui_ctx);
                        }

                        fn playable_definition(&mut self) -> &mut Definitions {
                            self.raw_definition()
                        }

                        fn playable_implementation(&mut self) -> &mut Option<Implementations> {
                            self.raw_implementation()
                        }

                        fn handle_input_for_playable(
                            &mut self,
                            settings: &GameProgrammeSettings,
                            state: &mut GameProgrammeState<InputContextEnum>,
                            window: &Arc<Window>,
                        ) {
                            self.handle_input_for_context(settings, state, window)
                        }
                    }

                };
            }
        }
        syn::Data::Union(_) => panic!("enums only"),
    }

    tokens.into()
}
*/
/*
#[proc_macro_derive(HandlesInputContext)]
pub fn derive_input_context_partial(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident;

    let tokens = quote! {
        impl HandlesInputContexts for #ident {
            fn handle_input_for_context (&mut self,
                                                         settings: &GameProgrammeSettings,
                                                         state: &mut GameProgrammeState,
                                                         window: &Arc<Window>,
            ) {

                self.handle_input(
                    settings,
                    state,
                    window);
            }

        }
    };

    tokens.into()
}

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
