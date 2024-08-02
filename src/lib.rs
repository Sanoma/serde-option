pub use serde_option_macros::serde_option;

// This module structure exists to allow unit tests. Currently it's not possible
// to run unit tests inside `proc-macro` crates, i.e. crates that export procedural macros.
#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_compile_failures() {
        // We use `trybuild` to test whether the macro raises compile errors as expected
        let t = trybuild::TestCases::new();
        t.compile_fail("test/ui/invalid_item_type.rs");
        t.compile_fail("test/ui/non_option_nullable.rs");
        t.compile_fail("test/ui/non_option_not_required.rs");
        t.compile_fail("test/ui/skip_nullable.rs");
        t.compile_fail("test/ui/skip_not_required.rs");
        t.compile_fail("test/ui/default_not_required.rs");
    }

    #[test]
    fn test_roundtrip_serialization() {
        use serde::{Deserialize, Serialize};

        #[serde_option]
        #[derive(Deserialize, Serialize, PartialEq, Debug)]
        struct Example {
            #[nullable]
            nullable: Option<u64>,
            #[not_required]
            optional: ::core::option::Option<u64>,
            #[nullable]
            #[not_required]
            both: ::std::option::Option<Option<u64>>,
        }

        let accepted = [
            json!({"nullable": 1}),
            json!({"nullable": null}),
            json!({"nullable": 1, "optional": 2}),
            json!({"nullable": 1, "optional": 2, "both": null}),
            json!({"nullable": 1, "optional": 2, "both": 3}),
        ];

        for json in accepted {
            let string_value = json.to_string();

            let model: Example =
                serde_json::from_str(&string_value).expect("Deserialization should work");

            let serialized = serde_json::to_string(&model).expect("Serialization should work");

            assert_eq!(
                model,
                serde_json::from_str(&serialized).expect("Roundtrip should work"),
                "Roundtrip should be equal"
            );
        }
    }

    #[test]
    fn test_default_fields() {
        use serde::{Deserialize, Serialize};

        #[serde_option]
        #[derive(Deserialize, Serialize, PartialEq, Debug)]
        struct Example {
            #[serde(default = "default_fn")]
            #[nullable]
            nullable_default: Option<String>,
        }
        fn default_fn() -> Option<String> {
            Some("hello".into())
        }

        let without: Example = serde_json::from_value(json!({})).expect("Accepts without value");
        assert_eq!(
            without,
            Example {
                nullable_default: default_fn()
            },
            "Should be default when not given"
        );

        let with_null: Example =
            serde_json::from_value(json!({"nullable_default": null})).expect("Accepts with null");

        assert_eq!(
            with_null,
            Example {
                nullable_default: None
            },
            "Should be null when given null"
        );

        let with_value: Example = serde_json::from_value(json!({"nullable_default": "value"}))
            .expect("Accepts with value");
        assert_eq!(
            with_value,
            Example {
                nullable_default: Some("value".into())
            },
            "Should be non-null when given non-null"
        );
    }

    #[test]
    fn test_skipped() {
        use serde::{Deserialize, Serialize};

        #[serde_option]
        #[derive(Deserialize, Serialize, PartialEq, Debug)]
        struct Example {
            #[serde(skip)]
            skipped: u64,
        }

        let example = Example { skipped: 42 };

        assert_eq!(
            &serde_json::to_string(&example).expect("Serialization should work"),
            "{}",
            "Skipped fields should not be serialized"
        )
    }

    #[cfg(feature = "utoipa")]
    #[test]
    fn test_utoipa_features() {
        use serde::{Deserialize, Serialize};
        use utoipa::openapi::{RefOr, Schema};
        use utoipa::ToSchema;

        #[serde_option(utoipa)]
        #[derive(Deserialize, Serialize, ToSchema, PartialEq, Debug)]
        struct Example {
            #[nullable]
            nullable_field: Option<u64>,
            #[not_required]
            not_required_field: Option<u64>,
        }

        let (_, schema) = Example::schema();
        let RefOr::T(Schema::Object(object)) = schema else {
            panic!("schema should be an object")
        };
        let Some(RefOr::T(Schema::Object(nullable_field))) =
            object.properties.get("nullable_field")
        else {
            panic!("nullable_field should exist and be an object")
        };
        let Some(RefOr::T(Schema::Object(not_required_field))) =
            object.properties.get("not_required_field")
        else {
            panic!("not_required_field should exist and be an object")
        };
        assert!(
            nullable_field.nullable,
            "nullable_field should be marked as nullable"
        );
        assert!(
            !not_required_field.nullable,
            "not_required_field should not be marked as nullable"
        );
        assert_eq!(
            &object.required,
            &["nullable_field"],
            "only nullable_field should be marked as required"
        );
    }
}
