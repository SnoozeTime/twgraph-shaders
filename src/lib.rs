#![recursion_limit = "512"]

extern crate proc_macro;
#[macro_use]
extern crate quote;


use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{Ident, LitInt, Token, LitStr, bracketed, braced, parenthesized};
use syn::parse::{Parse, ParseStream, Result};
use std::fs::File;
use std::io::Read;

mod descriptor;
mod push_constants;
use crate::descriptor::{generate_descriptor_layout, DescriptorInput};
use crate::push_constants::{PushConstants, generate_pc};

// TODO Whatever I use at the moment. Other to be implemented later :)
#[derive(Debug, Clone, Copy)]
enum ShaderKind {
    Vertex,
    Fragment,
}

impl ShaderKind {
    
    pub fn from_str(repr: &str) -> Self {
        match repr {
            "fragment" => ShaderKind::Fragment,
            "vertex" => ShaderKind::Vertex,
            _ => panic!(format!("Shader kind {} not supported yet.", repr))
        }
    }

    pub fn get_shaderc_kind(&self) -> shaderc::ShaderKind {

        match *self {
            ShaderKind::Vertex => shaderc::ShaderKind::Vertex,
            ShaderKind::Fragment => shaderc::ShaderKind::Fragment,
        }
    }


    pub fn generate_shaderstage(&self) -> proc_macro2::TokenStream {

        match *self {
            ShaderKind::Vertex => {
                quote!(ShaderStages { vertex: true, ..ShaderStages::none() })
            },
            ShaderKind::Fragment => {
                quote!(ShaderStages { fragment: true, ..ShaderStages::none() })    
            }
        }
    }

    pub fn generate_graphic_shader_type(&self) -> proc_macro2::TokenStream {
        match *self {
            ShaderKind::Vertex => {
                quote!(GraphicsShaderType::Vertex)
            },
            ShaderKind::Fragment => {
                quote!(GraphicsShaderType::Fragment)    
            }
        }
    }
}

struct MacroInput {
    path: String,
    kind: ShaderKind,
    input_desc: Vec<InterfaceElement>,
    output_desc: Vec<InterfaceElement>,

    // The size of each push constant range.
    push_constants: Option<PushConstants>,
    descriptors: Vec<DescriptorInput>,
}

impl Parse for MacroInput {

    fn parse(input: ParseStream) -> Result<Self> {

        let mut path = None;
        let mut kind = None;
        let mut input_desc = Vec::new();
        let mut output_desc = Vec::new();
        let mut push_constants = None;
        let mut descriptors = Vec::new();


        while !input.is_empty() {

            // path: "...",
            // kind: "....",
            let name: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            match name.to_string().as_ref() {
                "path" => {
                    if path.is_some() {
                        panic!("Only one path can be defined");
                    }

                    let path_value: LitStr = input.parse()?;
                    path = Some(path_value);
                },
                "kind" => {
                    if kind.is_some() {
                        panic!("Only one kind can be defined");
                    }

                    let kind_value: LitStr = input.parse()?;
                    kind = Some(ShaderKind::from_str(kind_value.value().as_str()));
                },
                "input" => {
                    let in_brackets;
                    bracketed!(in_brackets in input);

                    while !in_brackets.is_empty() {
                        let input_el: InterfaceElement = in_brackets.parse()?;

                        input_desc.push(input_el);

                        if !in_brackets.is_empty() {
                            in_brackets.parse::<Token![,]>()?;
                        }
                    }

                },
                "output" => {
                    let in_brackets;
                    bracketed!(in_brackets in input);

                    while !in_brackets.is_empty() {
                        let output_el: InterfaceElement = in_brackets.parse()?;

                        output_desc.push(output_el);

                        if !in_brackets.is_empty() {
                            in_brackets.parse::<Token![,]>()?;
                        }
                    }
                },
                "push_constants" => {

                    if push_constants.is_some() {
                        panic!("Only one push constant can be defined");
                    }

                    let pc: PushConstants = input.parse()?;
                    push_constants = Some(pc);
                },
                "descriptors" => {
                    let in_brackets;
                    bracketed!(in_brackets in input);

                    while !in_brackets.is_empty() {
                        descriptors.push(in_brackets.parse::<DescriptorInput>()?);
                        if !in_brackets.is_empty() {
                            in_brackets.parse::<Token![,]>()?;
                        }
                    }

                },
                _ => panic!("Unexpected value"),
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(MacroInput {
            kind: kind.expect("Cannot find shader kind"),
            path: path.expect("Cannot find shader path").value(),
            input_desc,
            output_desc,
            push_constants,
            descriptors,
        })
    }
}

struct InterfaceElement {
    format: Ident,
    name: LitStr,
}


impl Parse for InterfaceElement {

    fn parse(input: ParseStream) -> Result<Self> {
        let mut format = None;
        let mut name = None;
        let in_braces;
        braced!(in_braces in input);

        while !in_braces.is_empty() {

            let ident: Ident = in_braces.parse()?;
            in_braces.parse::<Token![:]>()?;

            match ident.to_string().as_ref() {
                "format" => {
                    if format.is_some() {
                        panic!("already has a format.");
                    }

                    let format_value: Ident = in_braces.parse()?;
                    format = Some(format_value);
                },
                "name" => {
                    if name.is_some() {
                        panic!("Already has a name.");
                    }

                    let name_value: LitStr = in_braces.parse()?;
                    name = Some(name_value);
                },
                _ => panic!("not expected"),
            }

            if !in_braces.is_empty() {
                in_braces.parse::<Token![,]>()?;
            }
        }

        Ok(Self {
            format: format.expect("Cannot find Shader interface format"),
            name: name.expect("Cannot find shader interface name"),
         })
    }
}

fn generate_interface(struct_name: Ident, elements: &Vec<InterfaceElement>) -> proc_macro2::TokenStream {

    let mut input_impl = vec!();
    if elements.len() > 0 {
        let mut current_num = (elements.len() - 1) as u16;
        for (index, element) in elements.iter().enumerate() {

            let index = index as u32;
            let name = &element.name;
            let format = &element.format;
            let next_index = index + 1;
            input_impl.push(quote!(
                    if self.0 == #current_num {
                        self.0 += 1;
                        return Some(ShaderInterfaceDefEntry {
                            location: #index..#next_index,
                            format: Format::#format,
                            name: Some(Cow::Borrowed(#name))
                        })
                    }
            ));

            if current_num > 0 {
                current_num = current_num - 1;
            }
        }
    }

    let mut iter_name = struct_name.to_string();
    iter_name.push_str("Iter");
    let iter_name = Ident::new(iter_name.as_str(), Span::call_site());

    let length = elements.len();
    quote!(
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
        pub struct #struct_name;

        unsafe impl ShaderInterfaceDef for #struct_name {
            type Iter = #iter_name;

            fn elements(&self) -> #iter_name {
                #iter_name(0)
            }
        }

        #[derive(Debug, Copy, Clone)]
        pub struct #iter_name(u16);
        impl Iterator for #iter_name {
            type Item = ShaderInterfaceDefEntry;

            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                #( #input_impl )*
                None
            }

            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                let len = #length - self.0 as usize;
                (len, Some(len))
            }
        }

        impl ExactSizeIterator for #iter_name { }
        )
}




fn compile(path: String, shader_kind: ShaderKind) -> Vec<u32> {
    let mut f = File::open(&path).unwrap();
    let mut content = String::new();
    f.read_to_string(&mut content).unwrap();

    let mut compiler = shaderc::Compiler::new().unwrap();
    compiler.compile_into_spirv(
        content.as_str(),
        shader_kind.get_shaderc_kind(),
        &path, "main", None).unwrap().as_binary().to_vec()
}


#[proc_macro]
pub fn twshader(input: TokenStream) -> TokenStream {
    let MacroInput { 
        path,
        kind,
        input_desc,
        output_desc,
        push_constants,
        descriptors } = syn::parse_macro_input!(input as MacroInput);

    // Compile to SPIRV :D
    let spirv = compile(path.clone(), kind);
    let path = LitStr::new(&path, Span::call_site());

    let struct_name_in = Ident::new("MainInput", Span::call_site());
    let in_interface = generate_interface(struct_name_in.clone(), &input_desc);
    let struct_name_out = Ident::new("MainOutput", Span::call_site());
    let out_interface = generate_interface(struct_name_out.clone(), &output_desc);
    let (pc_impl, pc_struct_impl) = generate_pc(push_constants);
    let (desc_impl, desc_struct_impl) = generate_descriptor_layout(descriptors);

    let shader_stage = kind.generate_shaderstage();
    let graphic_shader_type = kind.generate_graphic_shader_type();

    let shaderc_type = match kind.get_shaderc_kind() {
        shaderc::ShaderKind::Vertex => {
            Ident::new("Vertex", Span::call_site())
        },
        shaderc::ShaderKind::Fragment => {
            Ident::new("Fragment", Span::call_site())
        },
        _ => panic!("Not supported yet."),
    };

    let expanded = quote!(
        //use shaderc::{Compiler, CompileOptions};
        use std::fs::File;
        use std::io::Read;
        use vulkano::format::Format;
        use std::borrow::Cow;
        use vulkano::descriptor::descriptor::{DescriptorDescTy, DescriptorDesc, DescriptorBufferDesc, DescriptorImageDesc, DescriptorImageDescArray, DescriptorImageDescDimensions};
        use std::ffi::CStr;
        use vulkano::pipeline::shader::{GraphicsShaderType, ShaderInterfaceDef, ShaderInterfaceDefEntry, ShaderModule};
        use vulkano::descriptor::descriptor::ShaderStages;
        use vulkano::descriptor::pipeline_layout::PipelineLayoutDesc;
        use vulkano::descriptor::pipeline_layout::PipelineLayoutDescPcRange;
        use vulkano::pipeline::shader::GraphicsEntryPointAbstract;

        use vulkano::device::Device;
        use std::sync::Arc;


        #in_interface
        #out_interface

        // This structure describes layout of this stage.
        #[derive(Debug, Copy, Clone)]
        pub struct MainLayout(ShaderStages);
        unsafe impl PipelineLayoutDesc for MainLayout {
            // Number of descriptor sets it takes.
            #desc_impl
            #pc_impl
        }


        pub struct Shader {
            module: Arc<ShaderModule>,
        }


        impl Shader {

            pub fn load(device: Arc<Device>) -> Result<Self, vulkano::OomError> {
                let words = [ #( #spirv ),* ];

                unsafe {
                    Ok(
                        Shader {
                            module: ShaderModule::from_words(device, &words)?
                        })
                }
            }

            pub fn main_entry_point(&self) -> vulkano::pipeline::shader::GraphicsEntryPoint<(), MainInput, MainOutput, MainLayout> {
                unsafe { 
                    self.module.graphics_entry_point(
                        CStr::from_bytes_with_nul_unchecked(b"main\0"),
                        #struct_name_in,
                        #struct_name_out,
                        MainLayout(#shader_stage),
                        #graphic_shader_type
                    ) }
            }

            /// Reload the file and compile it to spirv again.
            pub fn recompile(&mut self, device: Arc<Device>) -> Result<(), Box<std::error::Error>> {
                let mut f = File::open(#path)?;
                let mut content = String::new();
                f.read_to_string(&mut content)?;

                let mut compiler = shaderc::Compiler::new().unwrap();
                let spirv = compiler.compile_into_spirv(
                    content.as_str(),
                    shaderc::ShaderKind::#shaderc_type,
                    #path, "main", None)?;

                let spirv = spirv.as_binary();

                //// then, change the module.
                unsafe {
                    self.module = ShaderModule::from_words(device, &spirv)?;
                }
                Ok(())
            }
        }

        pub mod ty {
            #pc_struct_impl
            #desc_struct_impl
        }


        );


        expanded.into()

}
