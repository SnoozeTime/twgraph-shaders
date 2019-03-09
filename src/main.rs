
mod vs {
    twgraph_shader::twshader!{
        path: "test.frag",
        kind: "fragment",
        input: [
            // This is the position in 2d space of the GUI
            {
                format: R32G32Sfloat,
                name: "position",
            },
            // This is the texture coords
            {
                format: R32G32Sfloat,
                name: "uv",
            },
            // This is the color :)
            {
                format: R32G32B32A32Sfloat,
                name: "color"
            }
        ],
        output: [


        ],
        push_constants: {
            name: PushConstants,
            ranges: [(color, 4), (color2, 4), (long_ass, 12), (color3, 4)],
        },
        descriptors: [
            {
                name: UBO,
                ty: Buffer,
                data: [(model, "mat4"), (view, "mat4"), (proj, "mat4")],
                binding: 0,
                set: 0,
            }
            
        ],
    }
}

fn main() {
    println!("hi");
}
