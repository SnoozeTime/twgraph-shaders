
mod vs {
    twgraph_shader::twshader!{
        path: "test.frag",
        kind: "hi",
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
        push_constants: [4, 4, 12, 4],
    }
}

fn main() {
    println!("hi");
}
