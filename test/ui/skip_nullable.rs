use serde::{Deserialize, Serialize};
use serde_option::serde_option;
use utoipa::ToSchema;

#[serde_option]
#[derive(Deserialize, Serialize, ToSchema)]
struct Foo {
    #[nullable]
    #[serde(skip)]
    x: Option<u64>,
}

fn main() {}
