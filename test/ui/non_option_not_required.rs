use serde_option::serde_option;

#[serde_option]
struct Foo {
    #[not_required]
    x: u64,
}

fn main() {}
