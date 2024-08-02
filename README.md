# serde_option

This library is designed to make nullable & non-required fields easier to work with using the serde library.

## Example
See the documentation for the `#[serde_option]` macro for detailed usage.

The following data model:

```rust
use serde::Serialize;
use serde_option::serde_option;

#[serde_option] // <-- Put this before the #[derive]
#[derive(Serialize)]
struct Data {
    #[nullable]
    nullable_field: Option<String>,
    #[not_required]
    not_required_field: Option<u64>,
    #[nullable]
    #[not_required]
    nullable_and_not_required_field: Option<Option<String>>,
    #[nullable]
    #[serde(default)]
    nullable_with_default: Option<String>,
    #[serde(skip)]
    skipped_field: Option<bool>,
}
```

... is equivalent to the following struct definition:

```rust
use serde::Serialize;
use serde_option::serde_option;

#[derive(Serialize)]
struct Data {
    #[serde(with = "Option")]
    nullable_field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(with = "serde_with::rust::unwrap_or_skip")]
    not_required_field: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(with = "serde_with::rust::double_option")]
    nullable_and_not_required_field: Option<Option<String>>,
    #[serde(default)]
    nullable_with_default: Option<String>,
    #[serde(skip)]
    skipped_field: Option<bool>,
}
```