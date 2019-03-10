use syn::{Ident, Token, bracketed, braced, LitInt, parenthesized};
use syn::parse::{Parse, ParseStream, Result};


pub struct PushConstants {
    pub name: Ident,
    pub ranges: Vec<(Ident, usize)>,
}

impl Parse for PushConstants {

    /// Parse for example:
    /// `{
    ///     name: MyStructName,
    ///     ranges: [(color, 3), (scale: 2)],
    /// }`
    ///
    fn parse(input: ParseStream) -> Result<Self> {

        let mut name = None;
        let mut ranges = None;
        let in_braces;
        braced!(in_braces in input);

        while !in_braces.is_empty() {

            let ident: Ident = in_braces.parse()?;
            in_braces.parse::<Token![:]>()?;

            match ident.to_string().as_ref() {
                "name" => {
                    if name.is_some() {
                        panic!("Cannot parse 'name' twice");
                    }

                    name = Some(in_braces.parse::<Ident>()?);
                }
                "ranges" => {

                    if ranges.is_some() {
                        panic!("Cannot parse 'ranges' twice");
                    }
                    // Parse [ (color, 4), (color2, 3)]

                    let mut ranges_vec = vec![];
                    let in_brackets;
                    bracketed!(in_brackets in in_braces);

                    while !in_brackets.is_empty() {

                        let in_parens;
                        parenthesized!(in_parens in in_brackets);


                        let range_ident: Ident = in_parens.parse()?;
                        in_parens.parse::<Token![,]>()?;
                        let range_size: LitInt = in_parens.parse()?;

                        if !in_parens.is_empty() {
                            panic!("Expected only tuple here");
                        }

                        ranges_vec.push((range_ident, range_size.value() as usize));
                        if !in_brackets.is_empty() {
                            in_brackets.parse::<Token![,]>()?;
                        }
                    }

                    ranges = Some(ranges_vec);
                }
                _ => panic!("unexpected"),
            }

            if !in_braces.is_empty() {
                in_braces.parse::<Token![,]>()?;
            }
        }

        Ok(PushConstants {
            name: name.expect("Cannot find push constants name"),
            ranges: ranges.expect("Cannot find push constants ranges"),
        })

    }
}

/// Return the pipeline layout and the data structure that represent this push constants.
pub fn generate_pc(pc: Option<PushConstants>) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {

    if let Some(pc) = pc {
        let length = pc.ranges.len();

        let mut inner_desc = vec![];
        let mut offset = 0usize;
        let mut struct_content = vec![];
        for (_, (field, size)) in pc.ranges.iter().enumerate() {

            let size_in_bytes = size * 4;
            struct_content.push(quote!(
                    pub #field: [f32; #size],
                    ));
            offset += size_in_bytes;
        }

        let struct_name = pc.name;
        let structure = quote!(

            #[repr(C)]
            #[derive(Copy, Clone)]
            pub struct #struct_name {
                #( #struct_content)*
            }

        );
        inner_desc.push(quote!(

                if num == 0 {

                    return Some(PipelineLayoutDescPcRange {
                        offset: 0,
                        size: #offset,
                        stages: ShaderStages::all(),
                    });
                }

        ));


        (quote!(
                // Number of push constants ranges (think: number of push constants).
                fn num_push_constants_ranges(&self) -> usize { 1 }
                // Each push constant range in memory.
                fn push_constants_range(&self, num: usize) -> Option<PipelineLayoutDescPcRange> { 


                    #( #inner_desc )*

                    None
                }
        ), structure)
    } else {

        (quote!(
                // Number of push constants ranges (think: number of push constants).
                fn num_push_constants_ranges(&self) -> usize { 0 }
                // Each push constant range in memory.
                fn push_constants_range(&self, num: usize) -> Option<PipelineLayoutDescPcRange> { 
                    None
                }
        ), quote!())
    }


}
