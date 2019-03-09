#![recursion_limit = "512"]

extern crate proc_macro;
#[macro_use]
extern crate quote;


use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{Ident, LitInt, Token, LitStr, bracketed, braced};
use syn::parse::{Parse, ParseStream, Result};
use std::fs::File;
use std::io::Read;

struct MacroInput {
    path: String,
    kind: String,
    input_desc: Vec<InterfaceElement>,
    output_desc: Vec<InterfaceElement>,

    // The size of each push constant range.
    push_constants: Vec<LitInt>,
    descriptors: Vec<String>,
}

impl Parse for MacroInput {

    fn parse(input: ParseStream) -> Result<Self> {

        let mut path = None;
        let mut kind = None;
        let mut input_desc = Vec::new();
        let mut output_desc = Vec::new();
        let mut push_constants = Vec::new();
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
                    kind = Some(kind_value);
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
                    let in_brackets;
                    bracketed!(in_brackets in input);
                    while !in_brackets.is_empty() {

                        let size: LitInt = in_brackets.parse()?;
                        push_constants.push(size);

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
            kind: kind.unwrap().value(),
            path: path.unwrap().value(),
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
            format: format.unwrap(),
            name: name.unwrap(),
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


fn generate_pc(sizes: Vec<LitInt>) -> proc_macro2::TokenStream {

    let length = sizes.len();

    let mut inner_desc = vec![];
    let mut offset = 0usize;
    for (idx, size) in sizes.iter().enumerate() {

        let size = size.value() as usize;

        inner_desc.push(quote!(

                if num == #idx {

                    return Some(PipelineLayoutDescPcRange {
                        offset: #offset,
                        size: #size,
                        stages: ShaderStages::all(),
                    });
                }

        ));

        offset += size;
    }

    quote!(
        // Number of push constants ranges (think: number of push constants).
        fn num_push_constants_ranges(&self) -> usize { #length }
        // Each push constant range in memory.
        fn push_constants_range(&self, num: usize) -> Option<PipelineLayoutDescPcRange> { 


            #( #inner_desc )*

            None
        }
    )


}

fn compile(path: String) -> Vec<u32> {
    let mut f = File::open(&path).unwrap();
    let mut content = String::new();
    f.read_to_string(&mut content).unwrap();

    let mut compiler = shaderc::Compiler::new().unwrap();
    compiler.compile_into_spirv(
        content.as_str(),
        shaderc::ShaderKind::Fragment,
        &path, "main", None).unwrap().as_binary().to_vec()
}


#[proc_macro]
pub fn twshader(input: TokenStream) -> TokenStream {
    let MacroInput { 
        path,
        kind,
        input_desc,
        output_desc,
        push_constants, ..} = syn::parse_macro_input!(input as MacroInput);

    // Compile to SPIRV :D
    let spirv = compile(path.clone());
    let path = LitStr::new(&path, Span::call_site());

    let struct_name_in = Ident::new("MainInput", Span::call_site());
    let in_interface = generate_interface(struct_name_in.clone(), &input_desc);
    let struct_name_out = Ident::new("MainOutput", Span::call_site());
    let out_interface = generate_interface(struct_name_out.clone(), &output_desc);
    let pc_impl = generate_pc(push_constants);

    let expanded = quote!(
        //use shaderc::{Compiler, CompileOptions};
        use std::fs::File;
        use std::io::Read;
        use vulkano::format::Format;
        use std::borrow::Cow;
        use vulkano::descriptor::descriptor::DescriptorDesc;
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
            fn num_sets(&self) -> usize { 0 }
            // Number of entries (bindings) in each set.
            fn num_bindings_in_set(&self, _set: usize) -> Option<usize> { None }
            // Descriptor descriptions.
            fn descriptor(&self, _set: usize, _binding: usize) -> Option<DescriptorDesc> { None }
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
                        MainLayout(ShaderStages { fragment: true, ..ShaderStages::none() }),
                        GraphicsShaderType::Fragment
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
                    shaderc::ShaderKind::Fragment,
                    #path, "main", None).unwrap();

                let spirv = spirv.as_binary();

                //// then, change the module.
                unsafe {
                    self.module = ShaderModule::from_words(device, &spirv)?;
                }
                Ok(())
            }
        }


        );


        expanded.into()

}
