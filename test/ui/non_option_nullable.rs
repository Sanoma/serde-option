use serde_option::serde_option;

#[serde_option]
struct Foo {
    #[nullable]
    x: u64,
}

fn main() {}
