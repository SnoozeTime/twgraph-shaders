use twgraph_shader::hello;


#[hello]
fn wrapped() {}

#[test]
fn the_test() {
    println!("{}", wrapped());
}
