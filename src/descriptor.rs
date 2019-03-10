use syn::{Ident, LitStr, LitInt, braced, Token, parenthesized, bracketed};
use std::collections::{HashSet, HashMap};
use syn::parse::{Parse, ParseStream, Result};

pub enum DescriptorType {
    Buffer(BufferData),
    SampledImage,
}

pub struct BufferData {
    // Ident is the name of the field. String will be either:
    // float, vec2, vec3, vec4, mat2, mat3, mat4, ... glsl data
    data: Vec<(Ident, String)>
}

impl Parse for BufferData {

    fn parse(input: ParseStream) -> Result<Self> {

        let in_brackets;
        bracketed!(in_brackets in input);

        let mut data = Vec::new();

        while !in_brackets.is_empty() {

            let in_parens;
            parenthesized!(in_parens in in_brackets);


            let ident: Ident = in_parens.parse()?;
            in_parens.parse::<Token![,]>()?;
            let data_type: LitStr = in_parens.parse()?;

            if !in_parens.is_empty() {
                panic!("Expected only tuple here");
            }

            data.push((ident, data_type.value()));

            if !in_brackets.is_empty() {
                in_brackets.parse::<Token![,]>()?;
            }
        }

        Ok(BufferData {
            data,
        })
    }

}

/// This is parsed from the macro input
pub struct DescriptorInput {
    name: Ident,
    ty: DescriptorType,
    binding: usize,
    set: usize,
}

impl Parse for DescriptorInput {

    /// `{
    ///     ty: Buffer,
    ///     data: [(model, "vec3"), ...]
    /// }`
    fn parse(input: ParseStream) -> Result<Self> {

        let in_braces;
        braced!(in_braces in input);


        let mut data = None;
        let mut ty_str = None;
        let mut binding = None;
        let mut name = None;
        let mut set = None;
        while !in_braces.is_empty() {

            let key: Ident = in_braces.parse()?;
            in_braces.parse::<Token![:]>()?;

            match key.to_string().as_str() {
                "name" => {
                    if name.is_some() {
                        panic!("Cannot define 'name' twice");
                    }

                    name = Some(in_braces.parse::<Ident>()?);
                },
                "ty" => {
                    if ty_str.is_some() {
                        panic!("Cannot define 'ty' twice");
                    }

                    ty_str = Some(in_braces.parse::<Ident>()?);
                },
                "data" => {
                    if data.is_some() {
                        panic!("Cannot define 'data' twice");
                    }

                    data = Some(in_braces.parse::<BufferData>()?);
                },
                "binding" => {
                    if binding.is_some() {
                        panic!("Cannot define 'binding' twice");
                    }

                    binding = Some(in_braces.parse::<LitInt>()?.value() as usize);
                },
                "set" => {
                    if set.is_some() {
                        panic!("Cannot define 'set' twice");
                    }
                    set = Some(in_braces.parse::<LitInt>()?.value() as usize);
                },
                _ => panic!("Not expected"),
            }

            if !in_braces.is_empty() {
                in_braces.parse::<Token![,]>()?;
            }
        }

        let ty = match ty_str.expect("Could not find uniform type").to_string().as_ref() {
            "Buffer" => DescriptorType::Buffer(data.expect("Could not find uniform buffer data")),
            "SampledImage" => DescriptorType::SampledImage,
            _ => panic!("Descriptor type not supported"),
        };

        Ok(Self {
            name: name.expect("Could not find descriptor name"),
            ty,
            binding: binding.expect("could not find descriptor binding"),
            set: set.expect("Could not find descriptor set"),
        })
    }
}


pub fn generate_descriptor_layout(descriptor_inputs: Vec<DescriptorInput>) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {

    // first let's order by set and binding.
    // I'm tired, don't judge.
    let mut bindings_per_set = HashMap::new();
    for desc in descriptor_inputs.iter() {

        if !bindings_per_set.contains_key(&desc.set) {
            bindings_per_set.insert(desc.set, HashSet::new());
        }

        bindings_per_set.get_mut(&desc.set)
            .expect(&format!("Cannot find HashSet for set: {}", desc.set))
            .insert(desc.binding);
    }

    let num_set = bindings_per_set.len();

    let mut num_bindings = vec![];
    for (set, bindings) in &bindings_per_set {

        let binding_length = bindings.len();
        num_bindings.push(quote!(
                #set => Some(#binding_length),
                ));
    }

    let mut descriptor_desc = vec![];
    let mut descriptor_structs = vec![];
    for desc in descriptor_inputs.iter() {

        let set = desc.set;
        let binding = desc.binding;

        let ty = match &desc.ty {
            DescriptorType::Buffer(_) => {
                quote!(
                    DescriptorDescTy::Buffer(DescriptorBufferDesc {
                        dynamic: Some(false),
                        storage: false,
                    })
                )
            },
            DescriptorType::SampledImage => {
                quote!(
                    DescriptorDescTy::CombinedImageSampler(DescriptorImageDesc {
                        sampled: true,
                        dimensions: DescriptorImageDescDimensions::TwoDimensional,
                        format: None,
                        multisampled: false,
                        array_layers: DescriptorImageDescArray::NonArrayed,
                    })
                )
            },
        };
        descriptor_desc.push(quote!(

                (#set, #binding) => Some(DescriptorDesc {
                    ty: #ty,
                    array_count: 1u32,
                    stages: self.0.clone(),
                    readonly: true,
                }),
                ));

        let name = &desc.name;
        let mut fields = vec![];
        if let DescriptorType::Buffer(BufferData {data}) = &desc.ty {
            for (field_name, field_ty) in data {
                // ident and string.
                match field_ty.as_ref() {
                    "vec2" => {
                        fields.push(quote!(
                                pub #field_name: [f32; 2],
                                ));
                    },
                    "vec3" => {
                        fields.push(quote!(
                                pub #field_name: [f32; 3],
                                ));
                    },
                    "vec4" => {
                        fields.push(quote!(
                                pub #field_name: [f32; 4],
                                ));
                    },
                    "mat2" => {
                        fields.push(quote!(
                                pub #field_name: [[f32; 2]; 2],
                                ));
                    },
                    "mat3" => {
                        fields.push(quote!(
                                pub #field_name: [[f32; 3]; 3],
                                ));
                    },
                    "mat4" => {
                        fields.push(quote!(
                                pub #field_name: [[f32; 4]; 4],
                                ));
                    },
                    x => panic!(format!("Uniform field type {} not supported yet", x)),
                }
            }
        }

        if !fields.is_empty() {
            descriptor_structs.push(quote!(
                    #[repr(C)]
                    #[derive(Debug, Clone, Copy)]
                    pub struct #name {
                        #( #fields )*
                    }
            ));
        }
    }


    (quote!(

            fn num_sets(&self) -> usize {
                #num_set
            }

            fn num_bindings_in_set(&self, set: usize) -> Option<usize> {
                match set {
                    #( #num_bindings )*
                    _ => None,
                }
            }

            fn descriptor(&self, set: usize, binding: usize) -> Option<DescriptorDesc> {
                match (set, binding) {
                    #( #descriptor_desc )*
                    _ => None,
                }
            }
    ), quote!(#( #descriptor_structs )*))
}

