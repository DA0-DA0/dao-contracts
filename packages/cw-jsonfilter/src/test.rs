use std::collections::HashSet;

use crate::{decoder::{NoopDecoder, ProtobufDecoder}, CwJsonFilter, FilterFailure, FilterFatalError, FilterResult};
use cw_protobuf_registry::protobuf::decode_protobuf;
use prost_types::FileDescriptorSet;
use cw_protobuf_registry::protobuf::{
    base64_encode_protobuf, get_protobuf_messages,
};
use prost_reflect::{prost::Message, DescriptorPool};
use serde_json::json;

struct MockProtobufDecoder {
    file_descriptor_set: FileDescriptorSet,
}

impl ProtobufDecoder for MockProtobufDecoder {
    fn decode(&self, message_name: String, value: Vec<u8>) -> Result<serde_json::Value, String> {
        decode_protobuf(self.file_descriptor_set.clone(), message_name, value)
    }
}



#[test]
fn array_match() {
    assert!(CwJsonFilter::check(
        &json!({ "a": [1, 2, 3, 4] }),
        &json!({ "a": "not_an_array" })
    )
    .is_fail());

    assert!(
        CwJsonFilter::check(&json!({ "a": [1, 2, 3, 4]}), &json!({ "a": [1, 2, 3, 4]})).is_pass()
    );

    assert!(CwJsonFilter::check(&json!({ "a": [1, 2, 3, 4]}), &json!({ "a": [1, 2, 3]})).is_fail());

    assert!(CwJsonFilter::check(
        &json!({ "a": [1, 2, 3, 4]}),
        &json!({ "a": [1, 2, 3, 4, 5]})
    )
    .is_fail());
}

#[test]
fn array_element_match() {
    assert!(CwJsonFilter::check(
        &json!(
            { "a": { "$contains": 3 }}
        ),
        &json!({ "a": [1,2,3,4]})
    )
    .is_pass());
}

#[test]
fn array_any_str() {
    assert!(CwJsonFilter::check(
        &json!(
            { "a": { "$any": { "$contains": "world"} }}
        ),
        &json!({ "a": ["hello", "world"]})
    )
    .is_pass());
}

#[test]
fn array_any_sub() {
    assert!(CwJsonFilter::check(
        &json!(
            { "a": { "$any": { "key": { "$contains": "world"} }}}
        ),
        &json!({ "a": [{ "key": "hello" }, {"key": "world"}]})
    )
    .is_pass());
}

#[test]
fn array_all_nested() {
    assert!(CwJsonFilter::check(
        &json!(
            { "key": { "$all": { "$gt": 10} }}
        ),
        &json!({ "key": [1,2,3,4,5]})
    )
    .is_fail());

    assert!(CwJsonFilter::check(
        &json!(
            { "key": { "$all": { "$gt": 5} }}
        ),
        &json!({ "key": [6,7,8,9]})
    )
    .is_pass());
}

#[test]
fn array_all_direct() {
    assert!(CwJsonFilter::check(
        &json!(
            { "$all": { "$gt": 10} }
        ),
        &json!([1, 2, 3, 4, 5])
    )
    .is_fail());

    assert!(CwJsonFilter::check(
        &json!(
            { "$all": { "$gt": 5} }
        ),
        &json!([6, 7, 8, 9])
    )
    .is_pass());
}

#[test]
fn array_any_direct() {
    assert!(CwJsonFilter::check(
        &json!(
            { "$any": { "$contains": "world"} }
        ),
        &json!(["hello", "world"])
    )
    .is_pass());
}

#[test]
fn simple_mask() {
    assert!(CwJsonFilter::check(
        &json!(
            { "key": "value" }
        ),
        &json!({
            "key": "value",
            "num": 3
        })
    )
    .is_pass());
    assert!(CwJsonFilter::check(
        &json!(
            { "key": "value" }
        ),
        &json!({
            "key": "not_value",
            "num": 3
        })
    )
    .is_fail());
}

#[test]
fn nested_mask() {
    assert!(CwJsonFilter::check(
        &json!({
            "key": {
                "nested": "value"
            }
        }),
        &json!({
            "key": {
                "nested": "value"
            }
        })
    )
    .is_pass());
}

#[test]
fn not_equal() {
    assert!(CwJsonFilter::check(
        &json!({
            "key": {
                "$ne": "value"
            }
        }),
        &json!({ "key": "not_value"})
    )
    .is_pass());
}

#[test]
fn greater_than() {
    assert!(CwJsonFilter::check(
        &json!({
            "key": {
                "$gt": 5
            }
        }),
        &json!({ "key": 4 })
    )
    .is_fail());
    assert!(CwJsonFilter::check(
        &json!({
            "key": {
                "$gt": 5
            }
        }),
        &json!({ "key": 5 })
    )
    .is_fail());
    assert!(CwJsonFilter::check(
        &json!({
            "key": {
                "$gte": 5
            }
        }),
        &json!({ "key": 5 })
    )
    .is_pass());
    assert!(CwJsonFilter::check(
        &json!({
            "key": {
                "$gte": "def"
            }
        }),
        &json!({ "key": "abc" })
    )
    .is_fail());
    assert!(CwJsonFilter::check(
        &json!({
            "key": {
                "$gte": "def"
            }
        }),
        &json!({ "key": "def" })
    )
    .is_pass());
    assert!(CwJsonFilter::check(
        &json!({
            "key": {
                "$gt": "def"
            }
        }),
        &json!({ "key": "def" })
    )
    .is_fail());
    assert!(CwJsonFilter::check(
        &json!({
            "key": {
                "$gte": "def"
            }
        }),
        &json!({ "key": "ghi" })
    )
    .is_pass());
    assert!(CwJsonFilter::check(
        &json!({
            "key": {
                "$gte": "def"
            }
        }),
        &json!({ "key": 123 })
    )
    .is_fail());
}

#[test]
fn less_than() {
    assert!(CwJsonFilter::check(
        &json!({
            "key": {
                "$lt": 5
            }
        }),
        &json!({ "key": 6 })
    )
    .is_fail());
    assert!(CwJsonFilter::check(
        &json!({
            "key": {
                "$lt": 5
            }
        }),
        &json!({ "key": 5 })
    )
    .is_fail());
    assert!(CwJsonFilter::check(
        &json!({
            "key": {
                "$lte": 5
            }
        }),
        &json!({ "key": 5 })
    )
    .is_pass());
    assert!(CwJsonFilter::check(
        &json!({
            "key": {
                "$lte": "def"
            }
        }),
        &json!({ "key": "abc" })
    )
    .is_pass());
    assert!(CwJsonFilter::check(
        &json!({
            "key": {
                "$lte": "def"
            }
        }),
        &json!({ "key": "def" })
    )
    .is_pass());
    assert!(CwJsonFilter::check(
        &json!({
            "key": {
                "$lt": "def"
            }
        }),
        &json!({ "key": "def" })
    )
    .is_fail());
    assert!(CwJsonFilter::check(
        &json!({
            "key": {
                "$lte": "def"
            }
        }),
        &json!({ "key": "ghi" })
    )
    .is_fail());
    assert!(CwJsonFilter::check(
        &json!({
            "key": {
                "$lte": "abc"
            }
        }),
        &json!({ "key": 123 })
    )
    .is_fail());
}

#[test]
fn text_contains() {
    assert!(CwJsonFilter::check(
        &json!({
            "key": {
                "$contains": "world"
            }
        }),
        &json!({
            "key": "hello world"
        })
    )
    .is_pass());
}

#[test]
fn range_op() {
    assert!(CwJsonFilter::check(
        &json!({"key": { "$range": [18, 30] }}),
        &json!({
            "key": 20
        })
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({"key": { "$range": [18, 30] }}),
        &json!({
            "key": 15
        })
    )
    .is_fail());

    assert!(CwJsonFilter::check(
        &json!({"key": { "$range": [18, 30] }}),
        &json!({
            "key": 40
        })
    )
    .is_fail());

    // Decimal
    assert!(CwJsonFilter::check(
        &json!({"key": { "$range": [18.0, 30.0] }}),
        &json!({
            "key": 20.0
        })
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({"key": { "$range": [18.0, 30.0] }}),
        &json!({
            "key": 20
        })
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({"key": { "$range": [18, 30.1] }}),
        &json!({
            "key": 20.0
        })
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({"key": { "$range": [18.9, 30] }}),
        &json!({
            "key": 20.1
        })
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({"key": { "$range": [20.01, 30.1] }}),
        &json!({
            "key": 20.0
        })
    )
    .is_fail());
}

#[test]
fn range_op_asserts_order() {
    // range bounds equal to eachother
    let res = CwJsonFilter::check(
        &json!({"key": { "$range": [30.0, 30.0] }}),
        &json!({
            "key": 20
        }),
    );
    assert!(res.is_fatal());

    // descending range is not supported
    let res = CwJsonFilter::check(
        &json!({"key": { "$range": [31.0, 30.0] }}),
        &json!({
            "key": 30.5
        }),
    );
    assert!(res.is_fatal());
    assert_eq!(
        res.as_fatal().unwrap(),
        &FilterFatalError::InvalidFilter {
            reason: "$range args must be in ascending order".into(),
            filter_path: "@.key.$range".into(),
            obj_path: "@.key".into()
        }
    );

    // range defined from incompatible types
    let res = CwJsonFilter::check(
        &json!({"key": { "$range": [31.0, "???"] }}),
        &json!({
            "key": 30.5
        }),
    );
    assert!(res.is_fatal());
}

#[test]
fn key_not_found_filter_failure_formatting() {
    let res = CwJsonFilter::check(
        &json!({"not_the_key": { "$range": [31.0, 30.0] }}),
        &json!({
            "key": 30.5
        }),
    );
    assert!(res.is_fail());
    let filter_failure = match res {
        FilterResult::Fail(filter_failure) => filter_failure,
        _ => panic!(),
    };
    assert_eq!(
        filter_failure.to_string(),
        "Key not found at object path: `@.not_the_key` for filter path: `@.not_the_key.$range`"
    );
}

#[test]
fn filter_result_formatting() {
    let key = json!({
        "key": 30.5
    });

    let res_pass = CwJsonFilter::check(&json!({"key": { "$range": [30.0, 31.0] }}), &key);
    assert!(res_pass.is_pass());
    assert_eq!(format!("{}", res_pass), "Pass".to_string());

    let res_fail = CwJsonFilter::check(&json!({"not_the_key": { "$range": [30.0, 31.0] }}), &key);
    assert!(res_fail.is_fail());
    match res_fail.clone() {
        FilterResult::Fail(filter_failure) => {
            assert_eq!(format!("Fail: {}", filter_failure), res_fail.to_string())
        }
        _ => panic!(),
    };

    let res_fatal = CwJsonFilter::check(&json!({"key": { "$range": [31.0, 30.0] }}), &key);
    assert!(res_fatal.is_fatal());
    match res_fatal.clone() {
        FilterResult::Fatal(filter_fatal_error) => assert_eq!(
            format!("Fatal: {}", filter_fatal_error),
            res_fatal.to_string()
        ),
        _ => panic!(),
    };
}

#[test]
fn in_array_contains() {
    assert!(CwJsonFilter::check(
        &json!({
            "key": {
                "$contains": 3
            }
        }),
        &json!({
            "key": [1,2,3,4,5]
        })
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({
            "$not": {
            "key": {
                "$contains": 3
            }}
        }),
        &json!({
            "key": [1,2,4,5]
        })
    )
    .is_pass());
}

#[test]
fn array_contains() {
    assert!(CwJsonFilter::check(
        &json!({
            "key": {
                "$contains": 3
            }
        }),
        &json!({
            "key": [1,2,3,4,5]
        })
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({
            "key": {
                "$not": {
                    "$contains": 3
                }
            }
        }),
        &json!({
            "key": [1,2,4,5]
        })
    )
    .is_pass());
}

#[test]
fn and_op() {
    assert!(CwJsonFilter::check(
        &json!({
            "$and": [
                { "key": "value" },
                { "num": { "$gt": 5 }}
            ]
        }),
        &json!({
            "key": "value",
            "num": 7
        })
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({
            "$and": [
                { "key": "value" },
                { "num": { "$gt": 5 }}
            ]
        }),
        &json!({
            "key": "value",
            "num": 3
        })
    )
    .is_fail());
}

#[test]
fn or_op() {
    let filter = json!({
        "$or": [
            { "key": "value" },
            { "num": { "$gt": 5} }
        ]
    });
    assert!(CwJsonFilter::check(
        &filter,
        &json!({
            "key": "value",
            "num": 6
        })
    )
    .is_pass());
    assert!(CwJsonFilter::check(
        &filter,
        &json!({
            "key": "value",
            "num": 2
        })
    )
    .is_pass());
    assert!(CwJsonFilter::check(
        &filter,
        &json!({
            "key": "not_value",
            "num": 6
        })
    )
    .is_pass());
    assert!(CwJsonFilter::check(
        &filter,
        &json!({
            "key": "not_value",
            "num": 2
        })
    )
    .is_fail());
}

#[test]
fn negation() {
    assert!(CwJsonFilter::check(
        &json!({
            "num": { "$not": { "$gt": 5 }}
        }),
        &json!({
            "num": 3
        })
    )
    .is_pass());
}

#[test]
fn exists() {
    assert!(CwJsonFilter::check(
        &json!({
            "key": { "$exists": true }
        }),
        &json!({
            "key": "value"
        })
    )
    .is_pass());
    assert!(CwJsonFilter::check(
        &json!({
            "key": { "$exists": true }
        }),
        &json!({})
    )
    .is_fail());
    assert!(CwJsonFilter::check(
        &json!({
            "key": { "$exists": false }
        }),
        &json!({
            "key": "value"
        })
    )
    .is_fail());
    assert!(CwJsonFilter::check(
        &json!({
            "key": {
                "$or": [
                    { "$exists": true },
                    { "$exists": false }
                ]
            }
        }),
        &json!({})
    )
    .is_pass());
}

#[test]
fn len() {
    assert!(CwJsonFilter::check(
        &json!({
            "list": { "#len": 3 }
        }),
        &json!({
            "list": [1,2,3]
        })
    )
    .is_pass());
    assert!(CwJsonFilter::check(
        &json!({
            "obj": { "#len": 3 }
        }),
        &json!({
            "obj": {
                "first": 1,
                "second": 2,
                "third": 3
            }
        })
    )
    .is_pass());
    assert!(CwJsonFilter::check(
        &json!({
            "str": { "#len": 3 }
        }),
        &json!({
            "str": "123"
        })
    )
    .is_pass());
    assert!(CwJsonFilter::check(
        &json!({
            "list": { "#size": 5 }
        }),
        &json!({
            "list": [1,2,3]
        })
    )
    .is_fail());
    assert!(CwJsonFilter::check(
        &json!({
            "obj": { "#size": 5 }
        }),
        &json!({
            "obj": {
                "first": 1,
                "second": 2,
                "third": 3
            }
        })
    )
    .is_fail());
    assert!(CwJsonFilter::check(
        &json!({
            "str": { "#len": 5 }
        }),
        &json!({
            "str": "123"
        })
    )
    .is_fail());
}

#[test]
fn type_match() {
    assert!(CwJsonFilter::check(
        &json!({
            "key": { "$type": "number"}
        }),
        &json!({
            "key": 3
        })
    )
    .is_pass());
    assert!(CwJsonFilter::check(
        &json!({
            "key": { "$type": "string"}
        }),
        &json!({
            "key": "value"
        })
    )
    .is_pass());
    assert!(CwJsonFilter::check(
        &json!({
            "key": { "$type": "array"}
        }),
        &json!({
            "key": [1,2,3]
        })
    )
    .is_pass());
    assert!(CwJsonFilter::check(
        &json!({
            "key": { "$type": "object"}
        }),
        &json!({
            "key": {
                "nested": "value"
            }
        })
    )
    .is_pass());
}

#[test]
fn multiple_mask() {
    assert!(CwJsonFilter::check(
        &json!({
            "key": "value",
            "num": 3
        }),
        &json!({
            "key": "value",
            "num": 3,
            "extra": "value"
        })
    )
    .is_pass());
    assert!(CwJsonFilter::check(
        &json!({
            "key": "value",
            "num": 3
        }),
        &json!({
            "key": "value",
            "num": 5,
            "extra": "value"
        })
    )
    .is_fail());
}

#[test]
fn array_len_cmp() {
    let empty: Vec<i32> = vec![];
    let one = vec![1];
    let two = vec![1, 2];
    let many = vec![1, 2, 3, 4, 5, 6];

    let filter = json!({
        "list": {"#len": {"$gt": 0}}
    });

    assert!(CwJsonFilter::check(&filter, &json!({"list": empty})).is_fail());
    assert!(CwJsonFilter::check(&filter, &json!({"list": one})).is_pass());
    assert!(CwJsonFilter::check(&filter, &json!({"list": two})).is_pass());
    assert!(CwJsonFilter::check(&filter, &json!({"list": many})).is_pass());
}

#[test]
fn nested_modifier() {
    assert!(CwJsonFilter::check(
        &json!({
            "key": {
                "nested": {
                    "$gt": 5
                }
            }
        }),
        &json!({
            "key": { "nested": 7 }
        })
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({
            "key": {
                "nested": {
                    "$gt": 5
                }
            }
        }),
        &json!({
            "key": { "nested": 2 }
        })
    )
    .is_fail());
}

#[test]
fn implied_and() {
    assert!(CwJsonFilter::check(
        &json!({
            "number": {
                "$gt": 5,
                "$lt": 10
            }
        }),
        &json!({
            "number": 7
        })
    )
    .is_pass());
    assert!(CwJsonFilter::check(
        &json!({
            "number": {
                "$gt": 5,
                "$lt": 10
            }
        }),
        &json!({
            "number": 11
        })
    )
    .is_fail());

    assert!(CwJsonFilter::check(
        &json!({
            "$and": [
                { "number": { "$gt": 5 } },
                { "number": { "$lt": 10 } }
            ]
        }),
        &json!({
            "number": 7
        })
    )
    .is_pass());
    assert!(CwJsonFilter::check(
        &json!({
            "$and": [
                { "number": { "$gt": 5 } },
                { "number": { "$lt": 10 } }
            ]
        }),
        &json!({
            "number": 11
        })
    )
    .is_fail());

    assert!(CwJsonFilter::check(
        &json!({
            "number": {
                "$and": [
                    { "$gt": 5 },
                    { "$lt": 10 }
                ]
              }
        }),
        &json!({
            "number": 7
        })
    )
    .is_pass());
    assert!(CwJsonFilter::check(
        &json!({
            "number": {
                "$and": [
                    { "$gt": 5 },
                    { "$lt": 10 }
                ]
            }
        }),
        &json!({
            "number": 11
        })
    )
    .is_fail());
}

#[test]
fn nor_op() {
    let filter = json!({
        "$not": {
            "$or": [
                { "status": "banned" },
                { "status": "suspended" }
            ]
        }
    });

    assert!(CwJsonFilter::check(&filter, &json!({"status": "active"})).is_pass());
    assert!(CwJsonFilter::check(&filter, &json!({"status": "pending"})).is_pass());
    assert!(CwJsonFilter::check(&filter, &json!({"status": "banned"})).is_fail());
    assert!(CwJsonFilter::check(&filter, &json!({"status": "suspended"})).is_fail());
}

#[test]
fn xor_op() {
    let filter = json!({
        "$xor": [
            { "is_premium": true },
            { "is_trial": true }
        ]
    });

    // Exactly one should be true
    assert!(
        CwJsonFilter::check(&filter, &json!({"is_premium": true, "is_trial": false})).is_pass()
    );
    assert!(
        CwJsonFilter::check(&filter, &json!({"is_premium": false, "is_trial": true})).is_pass()
    );

    // Both true or both false should fail
    assert!(CwJsonFilter::check(&filter, &json!({"is_premium": true, "is_trial": true})).is_fail());
    assert!(
        CwJsonFilter::check(&filter, &json!({"is_premium": false, "is_trial": false})).is_fail()
    );
}

#[test]
fn eq_op() {
    assert!(CwJsonFilter::check(
        &json!({"status": {"$eq": "active"}}),
        &json!({"status": "active"})
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({"status": {"$eq": "active"}}),
        &json!({"status": "inactive"})
    )
    .is_fail());
}

#[test]
fn neq_op() {
    assert!(CwJsonFilter::check(
        &json!({"status": {"$neq": "deleted"}}),
        &json!({"status": "active"})
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({"status": {"$neq": "deleted"}}),
        &json!({"status": "deleted"})
    )
    .is_fail());
}

#[test]
fn range_exclusive_op() {
    let filter = json!({"temperature": {"$range_exclusive": [0, 100]}});

    assert!(CwJsonFilter::check(&filter, &json!({"temperature": 50})).is_pass());
    assert!(CwJsonFilter::check(&filter, &json!({"temperature": 0.1})).is_pass());
    assert!(CwJsonFilter::check(&filter, &json!({"temperature": 99.9})).is_pass());

    // Boundaries should fail (exclusive)
    assert!(CwJsonFilter::check(&filter, &json!({"temperature": 0})).is_fail());
    assert!(CwJsonFilter::check(&filter, &json!({"temperature": 100})).is_fail());
    assert!(CwJsonFilter::check(&filter, &json!({"temperature": -10})).is_fail());
    assert!(CwJsonFilter::check(&filter, &json!({"temperature": 110})).is_fail());
}

#[test]
fn between_exclusive_op() {
    let filter = json!({"percentage": {"$between_exclusive": [0, 1]}});

    assert!(CwJsonFilter::check(&filter, &json!({"percentage": 0.5})).is_pass());
    assert!(CwJsonFilter::check(&filter, &json!({"percentage": 0.001})).is_pass());
    assert!(CwJsonFilter::check(&filter, &json!({"percentage": 0.999})).is_pass());

    // Boundaries should fail (exclusive)
    assert!(CwJsonFilter::check(&filter, &json!({"percentage": 0})).is_fail());
    assert!(CwJsonFilter::check(&filter, &json!({"percentage": 1})).is_fail());
}

#[test]
fn overlap_op() {
    let filter = json!({"user_roles": {"$overlap": ["admin", "moderator"]}});

    assert!(CwJsonFilter::check(&filter, &json!({"user_roles": ["admin", "user"]})).is_pass());
    assert!(CwJsonFilter::check(&filter, &json!({"user_roles": ["moderator"]})).is_pass());
    assert!(CwJsonFilter::check(&filter, &json!({"user_roles": ["admin", "moderator"]})).is_pass());

    assert!(CwJsonFilter::check(&filter, &json!({"user_roles": ["user", "guest"]})).is_fail());
    assert!(CwJsonFilter::check(&filter, &json!({"user_roles": []})).is_fail());
}

#[test]
fn starts_with_op() {
    assert!(CwJsonFilter::check(
        &json!({"name": {"$startsWith": "Dr."}}),
        &json!({"name": "Dr. Smith"})
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({"url": {"$startsWith": "https://"}}),
        &json!({"url": "https://example.com"})
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({"name": {"$startsWith": "Dr."}}),
        &json!({"name": "Mr. Smith"})
    )
    .is_fail());
}

#[test]
fn ends_with_op() {
    assert!(CwJsonFilter::check(
        &json!({"filename": {"$endsWith": ".pdf"}}),
        &json!({"filename": "document.pdf"})
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({"email": {"$endsWith": "@company.com"}}),
        &json!({"email": "user@company.com"})
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({"filename": {"$endsWith": ".pdf"}}),
        &json!({"filename": "document.txt"})
    )
    .is_fail());
}

#[test]
fn size_op() {
    assert!(CwJsonFilter::check(
        &json!({"items": {"#size": 3}}),
        &json!({"items": [1, 2, 3]})
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({"items": {"#size": {"$gt": 2}}}),
        &json!({"items": [1, 2, 3, 4]})
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({"password": {"#size": {"$gte": 8}}}),
        &json!({"password": "mypassword"})
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({"password": {"#size": {"$gte": 8}}}),
        &json!({"password": "123"})
    )
    .is_fail());
}

#[test]
fn lower_transformation() {
    assert!(CwJsonFilter::check(
        &json!({"name": {"#lower": {"$eq": "john doe"}}}),
        &json!({"name": "John Doe"})
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({"email": {"#lower": {"$endsWith": "@gmail.com"}}}),
        &json!({"email": "USER@Gmail.Com"})
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({"name": {"#lower": {"$eq": "john doe"}}}),
        &json!({"name": "Jane Doe"})
    )
    .is_fail());
}

#[test]
fn upper_transformation() {
    assert!(CwJsonFilter::check(
        &json!({"code": {"#upper": {"$startsWith": "US"}}}),
        &json!({"code": "us-east-1"})
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({"country": {"#upper": {"$eq": "UNITED STATES"}}}),
        &json!({"country": "united states"})
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({"code": {"#upper": {"$startsWith": "US"}}}),
        &json!({"code": "eu-west-1"})
    )
    .is_fail());
}

#[test]
fn keys_transformation() {
    assert!(CwJsonFilter::check(
        &json!({"metadata": {"#keys": {"$contains": "version"}}}),
        &json!({"metadata": {"version": "1.0", "author": "John"}})
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({"config": {"#keys": {"#len": {"$gt": 0}}}}),
        &json!({"config": {"setting1": "value1"}})
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({"metadata": {"#keys": {"$contains": "version"}}}),
        &json!({"metadata": {"author": "John"}})
    )
    .is_fail());
}

#[test]
fn values_transformation() {
    assert!(CwJsonFilter::check(
        &json!({"scores": {"#values": {"$any": {"$gt": 95}}}}),
        &json!({"scores": {"math": 98, "science": 85}})
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({"settings": {"#values": {"$all": {"$type": "string"}}}}),
        &json!({"settings": {"name": "John", "email": "john@example.com"}})
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({"scores": {"#values": {"$any": {"$gt": 95}}}}),
        &json!({"scores": {"math": 85, "science": 80}})
    )
    .is_fail());
}

#[test]
fn base64_transformation() {
    // "hello world" in base64 is "aGVsbG8gd29ybGQ="
    assert!(CwJsonFilter::check(
        &json!({"data": {"#base64": {"$eq": "hello world"}}}),
        &json!({"data": "aGVsbG8gd29ybGQ="})
    )
    .is_pass());

    // JSON object {"name": "John"} in base64 is "eyJuYW1lIjoiSm9obiJ9"
    assert!(CwJsonFilter::check(
        &json!({"encoded_user": {"#base64": {"name": "John"}}}),
        &json!({"encoded_user": "eyJuYW1lIjoiSm9obiJ9"})
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &json!({"data": {"#base64": {"$eq": "hello world"}}}),
        &json!({"data": "invalid-base64"})
    )
    .is_fail());
}

#[test]
fn complex_nested_filter() {
    let filter = json!({
        "$and": [
            {
                "$or": [
                    {"age": {"$gte": 18}},
                    {"is_student": true}
                ]
            },
            {
                "$not": {
                    "$or": [
                        {"city": "New York"},
                        {"city": "Los Angeles"}
                    ]
                }
            }
        ]
    });

    // Adult in Chicago
    assert!(CwJsonFilter::check(
        &filter,
        &json!({
            "age": 25,
            "is_student": false,
            "city": "Chicago"
        })
    )
    .is_pass());

    // Student in Miami
    assert!(CwJsonFilter::check(
        &filter,
        &json!({
            "age": 16,
            "is_student": true,
            "city": "Miami"
        })
    )
    .is_pass());

    // Adult in banned city
    assert!(CwJsonFilter::check(
        &filter,
        &json!({
            "age": 30,
            "is_student": false,
            "city": "New York"
        })
    )
    .is_fail());

    // Minor non-student
    assert!(CwJsonFilter::check(
        &filter,
        &json!({
            "age": 16,
            "is_student": false,
            "city": "Chicago"
        })
    )
    .is_fail());
}

#[test]
fn array_element_filtering() {
    let filter = json!({
        "users": {
            "$any": {
                "$and": [
                    {"role": "admin"},
                    {"active": true}
                ]
            }
        }
    });

    assert!(CwJsonFilter::check(
        &filter,
        &json!({
            "users": [
                {"role": "user", "active": true},
                {"role": "admin", "active": true}
            ]
        })
    )
    .is_pass());

    assert!(CwJsonFilter::check(
        &filter,
        &json!({
            "users": [
                {"role": "user", "active": true},
                {"role": "admin", "active": false}
            ]
        })
    )
    .is_fail());
}

#[test]
fn complex_validation() {
    let filter = json!({
        "$and": [
            {"age": {"$type": "number"}},
            {"age": {"$range": [13, 120]}},
            {"name": {"$type": "string"}},
            {"name": {"#len": {"$range": [1, 100]}}},
        ]
    });

    assert!(CwJsonFilter::check(
        &filter,
        &json!({
            "age": 25,
            "name": "John Doe",
        })
    )
    .is_pass());

    // Invalid name
    assert!(CwJsonFilter::check(
        &filter,
        &json!({
            "age": 25,
            "name": 123,
        })
    )
    .is_fail());
    assert!(CwJsonFilter::check(
        &filter,
        &json!({
            "age": 25,
            "name": "",
        })
    )
    .is_fail());

    // Invalid age
    assert!(CwJsonFilter::check(
        &filter,
        &json!({
            "age": 150,
            "name": "John Doe",
        })
    )
    .is_fail());
    assert!(CwJsonFilter::check(
        &filter,
        &json!({
            "age": "50",
            "name": "John Doe",
        })
    )
    .is_fail());
}

#[test]
fn working_with_encoded_data() {
    // JWT payload example: {"user_id": "123", "exp": 1672531200}
    // Base64: eyJ1c2VyX2lkIjoiMTIzIiwiZXhwIjoxNjcyNTMxMjAwfQ==
    let filter = json!({
        "jwt_payload": {
            "#base64": {
                "$and": [
                    {"user_id": {"$type": "string"}},
                    {"exp": {"$gt": 1600000000}}
                ]
            }
        }
    });

    assert!(CwJsonFilter::check(
        &filter,
        &json!({
            "jwt_payload": "eyJ1c2VyX2lkIjoiMTIzIiwiZXhwIjoxNjcyNTMxMjAwfQ=="
        })
    )
    .is_pass());
}

#[test]
fn array_filters_combined() {
    let filter = json!({
        "$and": [
            {"tags": {"#len": {"$gte": 2}}},
            {"tags": {"$any": {"$startsWith": "tech"}}},
            {"tags": {"$all": {"#len": {"$gte": 3}}}}
        ]
    });

    assert!(CwJsonFilter::check(
        &filter,
        &json!({
            "tags": ["technology", "tech-news"]
        })
    )
    .is_pass());

    // Fails because "ai" is too short
    assert!(CwJsonFilter::check(
        &filter,
        &json!({
            "tags": ["technology", "ai"]
        })
    )
    .is_fail());
}

#[test]
fn edge_cases() {
    // Empty arrays and objects
    assert!(CwJsonFilter::check(
        &json!({"items": {"$all": {"$gt": 0}}}),
        &json!({"items": []})
    )
    .is_pass()); // $all passes on empty arrays

    assert!(CwJsonFilter::check(
        &json!({"items": {"$any": {"$gt": 0}}}),
        &json!({"items": []})
    )
    .is_fail()); // $any fails on empty arrays

    assert!(CwJsonFilter::check(&json!({"items": {"$and": []}}), &json!({"items": {}})).is_pass()); // $and passes on empty arrays

    assert!(CwJsonFilter::check(&json!({"items": {"$or": []}}), &json!({"items": {}})).is_fail()); // $or fails on empty arrays

    // Null values
    assert!(CwJsonFilter::check(
        &json!({"value": {"$type": "null"}}),
        &json!({"value": null})
    )
    .is_pass());

    // Boolean values
    assert!(CwJsonFilter::check(
        &json!({"active": {"$type": "boolean"}}),
        &json!({"active": true})
    )
    .is_pass());

    // Empty object
    assert!(CwJsonFilter::check(&json!({"anObject": {}}), &json!({})).is_fail());
    assert!(CwJsonFilter::check(
        &json!({"anObject": {
            "another": {}
        }}),
        &json!({})
    )
    .is_fail());
    assert!(CwJsonFilter::check(
        &json!({"anObject": {
            "another": {}
        }}),
        &json!({ "anObject": {}})
    )
    .is_fail());

    // Empty array
    assert!(CwJsonFilter::check(&json!({"anArray": []}), &json!({})).is_fail());
}

#[test]
fn protobuf_filter() {
    // Use CARGO_MANIFEST_DIR to get the crate root reliably
    let crate_root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let proto_path = std::path::Path::new(&crate_root).join("proto/string_bool_value.pb");

    let file_descriptor_set =
        FileDescriptorSet::decode(std::fs::read(proto_path).unwrap().as_slice()).unwrap();
    let pool = DescriptorPool::from_file_descriptor_set(file_descriptor_set.clone()).unwrap();

    let cwjf = CwJsonFilter::new(Some(MockProtobufDecoder { file_descriptor_set }));

    // String filter

    let string_filter =
        json!({"someProto": {"#proto": {"type": "google.protobuf.StringValue", "value": "pass"}}});
    let base64_encoded_pass =
        base64_encode_protobuf(&pool, "google.protobuf.StringValue", &json!("pass"));
    let base64_encoded_not_pass =
        base64_encode_protobuf(&pool, "google.protobuf.StringValue", &json!("not_test"));

    assert!(cwjf
        .matches(&string_filter, &json!({"someProto": base64_encoded_pass}))
        .is_pass());
    assert!(cwjf
        .matches(
            &string_filter,
            &json!({"someProto": base64_encoded_not_pass})
        )
        .is_fail());

    // Bool filter

    let bool_filter =
        json!({"someProto": {"#proto": {"type": "google.protobuf.BoolValue", "value": true}}});
    let base64_encoded_pass =
        base64_encode_protobuf(&pool, "google.protobuf.BoolValue", &json!(true));
    let base64_encoded_not_pass =
        base64_encode_protobuf(&pool, "google.protobuf.BoolValue", &json!(false));

    assert!(cwjf
        .matches(&bool_filter, &json!({"someProto": base64_encoded_pass}))
        .is_pass());
    assert!(cwjf
        .matches(&bool_filter, &json!({"someProto": base64_encoded_not_pass}))
        .is_fail());

    // Stargate shorthand

    let stargate_filter =
        json!({"#stargate": {"type_url": "/google.protobuf.StringValue", "value": "pass"}});
    let base64_encoded_pass =
        base64_encode_protobuf(&pool, "google.protobuf.StringValue", &json!("pass"));
    let base64_encoded_not_pass =
        base64_encode_protobuf(&pool, "google.protobuf.StringValue", &json!("not_test"));

    assert!(cwjf
        .matches(
            &stargate_filter,
            &json!({"stargate": {
                "type_url": "/google.protobuf.StringValue",
                "value": base64_encoded_pass
            }})
        )
        .is_pass());
    // missing / prefix
    assert!(cwjf
        .matches(
            &stargate_filter,
            &json!({"stargate": {
                "type_url": "google.protobuf.StringValue",
                "value": base64_encoded_pass
            }})
        )
        .is_fail());
    // wrong type URL
    assert!(cwjf
        .matches(
            &stargate_filter,
            &json!({"stargate": {
                "type_url": "/google.protobuf.WRONG.StringValue",
                "value": base64_encoded_pass
            }})
        )
        .is_fail());
    assert!(cwjf
        .matches(
            &stargate_filter,
            &json!({"stargate": {
                "type_url": "/google.protobuf.StringValue",
                "value": base64_encoded_not_pass
            }})
        )
        .is_fail());
}

#[test]
fn test_get_protobuf_messages() {
    let filter =
        json!({"someProto": {"#proto": {"type": "google.protobuf.StringValue", "value": "pass"}}});
    assert_eq!(
        get_protobuf_messages(&filter),
        HashSet::from(["google.protobuf.StringValue".to_string()])
    );

    let nested_filter = json!({"someProto": {"#proto": {"type": "google.protobuf.StringValue", "value": "pass"}, "someOtherProto": {"#proto": {"type": "google.protobuf.StringValue", "value": "pass"}}}});
    assert_eq!(
        get_protobuf_messages(&nested_filter),
        HashSet::from(["google.protobuf.StringValue".to_string(),])
    );

    let multiple_types_filter = json!({"someProto": {"#proto": {"type": "google.protobuf.StringValue", "value": "pass"}}, "someOtherProto": {"#proto": {"type": "google.protobuf.BoolValue", "value": true}}, "deeplyNested": {"somewhereFarFarAway": {"#proto": {"type": "another.proto.SomeMessage", "value": "pass"}}}, "invalidProto": {"#proto": {"noType": "getsIgnored"}}});
    assert_eq!(
        get_protobuf_messages(&multiple_types_filter),
        HashSet::from([
            "google.protobuf.StringValue".to_string(),
            "google.protobuf.BoolValue".to_string(),
            "another.proto.SomeMessage".to_string(),
        ])
    );

    let stargate_filter =
        json!({"#stargate": {"type_url": "/google.protobuf.StringValue", "value": "pass"}});
    assert_eq!(
        get_protobuf_messages(&stargate_filter),
        HashSet::from(["google.protobuf.StringValue".to_string()])
    );

    let nested_stargate_filter = json!({"#stargate": {"type_url": "/google.protobuf.StringValue", "value": "pass"}, "someOtherProto": {"#stargate": {"type_url": "/google.protobuf.BoolValue", "value": true}}});
    assert_eq!(
        get_protobuf_messages(&nested_stargate_filter),
        HashSet::from([
            "google.protobuf.StringValue".to_string(),
            "google.protobuf.BoolValue".to_string()
        ])
    );

    let multiple_stargate_types_filter = json!({"#stargate": {"type_url": "/google.protobuf.StringValue", "value": "pass"}, "someOtherProto": {"#stargate": {"type_url": "/google.protobuf.BoolValue", "value": true}, "deeplyNested": {"somewhereFarFarAway": {"#stargate": {"type_url": "/another.proto.SomeMessage", "value": "pass"}}}, "invalidProto": {"#stargate": {"noType": "getsIgnored"}}, "invalidProtoUrl": {"#stargate": {"type_url": "no.slash.prefix", "value": "getsIgnored"}}}});
    assert_eq!(
        get_protobuf_messages(&multiple_stargate_types_filter),
        HashSet::from([
            "google.protobuf.StringValue".to_string(),
            "google.protobuf.BoolValue".to_string(),
            "another.proto.SomeMessage".to_string(),
        ])
    );
}

#[test]
fn test_to_string() {
    let filter = json!({"score": {"#to_string": "100"}});
    assert!(CwJsonFilter::check(&filter, &json!({"score": "100"})).is_pass());
    assert!(CwJsonFilter::check(&filter, &json!({"score": 100})).is_pass());
    assert!(CwJsonFilter::check(&filter, &json!({"score": 99})).is_fail());
    assert!(CwJsonFilter::check(&filter, &json!({"score": "99"})).is_fail());
}

#[test]
fn test_to_number() {
    let filter = json!({"score": {"#to_number": {"$between": [25, 75]}}});
    assert!(CwJsonFilter::check(&filter, &json!({"score": "50"})).is_pass());
    assert!(CwJsonFilter::check(&filter, &json!({"score": 50})).is_pass());
    assert!(CwJsonFilter::check(&filter, &json!({"score": 99})).is_fail());
    assert!(CwJsonFilter::check(&filter, &json!({"score": "99"})).is_fail());
}

#[test]
fn test_replace() {
    let filter = json!({"duration": {"#replace": {"find": "s", "replace": "", "filter": {"#to_number": {"$gt": 0}}}}});
    assert!(CwJsonFilter::check(&filter, &json!({"duration": "1000s"})).is_pass());
    assert!(CwJsonFilter::check(&filter, &json!({"duration": "1000"})).is_pass());
    assert!(CwJsonFilter::check(&filter, &json!({"duration": "1000ms"})).is_fail());

    assert!(CwJsonFilter::check(
        &json!({ "key": { "#replace": { "find": "test", "replace": "tEsT", "filter": "atEsT" } } }),
        &json!({ "key": "atest" })
    )
    .is_pass());
}

#[test]
fn specific_array_filter() {
    let filter = json!({"anArray": {"1": "pass", "3": {
        "$or": [
            {"$exists": false },
            "item_4"
        ]
    }}});
    assert!(CwJsonFilter::check(&filter, &json!({"anArray": [1, 2, 3]})).is_fail());
    assert!(CwJsonFilter::check(&filter, &json!({"anArray": [1, 2, 3, "pass"]})).is_fail());
    assert!(CwJsonFilter::check(&filter, &json!({"anArray": [1, "pass", 3]})).is_pass());
    assert!(CwJsonFilter::check(&filter, &json!({"anArray": [1, "pass", 3, "item_4"]})).is_pass());
}

#[test]
fn key_not_found() {
    let filter = json!({"anObject": {}});
    assert_eq!(
        CwJsonFilter::check(&filter, &json!({})).as_fail().unwrap(),
        &FilterFailure::KeyNotFound {
            filter_path: "@.anObject".to_string(),
            obj_path: "@.anObject".to_string()
        }
    );

    // operators require a value
    assert_eq!(
        CwJsonFilter::check(&json!({ "key": { "$eq": "abc" } }), &json!({})),
        FilterResult::key_not_found("@.key.$eq", "@.key")
    );
}

#[test]
fn unknown_operator() {
    let filter = json!({"anObject": {"$unknownOperator": "pass"}});
    let result = CwJsonFilter::check(&filter, &json!({"anObject": "pass"}));
    assert!(result.is_fatal());
    assert_eq!(
        result.as_fatal().unwrap(),
        &FilterFatalError::UnknownOperator {
            operator: "$unknownOperator".to_string(),
            filter_path: "@.anObject.$unknownOperator".to_string(),
            obj_path: "@.anObject".to_string()
        }
    );
}

#[test]
fn invalid_operator_arg() {
    // $exists operator should be a boolean
    assert_eq!(
        CwJsonFilter::check(&json!({ "$exists": "abc" }), &json!({})),
        FilterResult::fatal_invalid_filter("$exists arg must be a boolean", "@.$exists", "@")
    );

    // $and operator should be an array
    assert_eq!(
        CwJsonFilter::check(&json!({ "$and": "abc" }), &json!({})),
        FilterResult::fatal_invalid_filter("$and arg must be an array", "@.$and", "@")
    );

    // $or operator should be an array
    assert_eq!(
        CwJsonFilter::check(&json!({ "$or": "abc" }), &json!({})),
        FilterResult::fatal_invalid_filter("$or arg must be an array", "@.$or", "@")
    );

    // $not operator should be an object
    assert_eq!(
        CwJsonFilter::check(&json!({ "$xor": "abc" }), &json!({})),
        FilterResult::fatal_invalid_filter("$xor arg must be an array", "@.$xor", "@")
    );

    // $xor operator should be an array
    assert_eq!(
        CwJsonFilter::check(&json!({ "$xor": "abc" }), &json!({})),
        FilterResult::fatal_invalid_filter("$xor arg must be an array", "@.$xor", "@")
    );

    // $range operator requires the same types
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "$range": [1, "abc"] } }),
            &json!({ "key": 1 })
        ),
        FilterResult::fatal_invalid_filter(
            "$range args must be both numbers or both strings",
            "@.key.$range",
            "@.key"
        )
    );
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "$range": [1, 3] } }),
            &json!({ "key": true })
        ),
        FilterResult::operator_failed(
            "$range",
            "filter bounds and value are not all numbers or all strings",
            "@.key.$range",
            "@.key"
        )
    );
    assert_eq!(
        CwJsonFilter::check(&json!({ "key": { "$range": true } }), &json!({ "key": 1 })),
        FilterResult::fatal_invalid_filter(
            "$range arg must be an array of two numbers or two strings",
            "@.key.$range",
            "@.key"
        )
    );

    // $type requires valid type
    assert_eq!(
        CwJsonFilter::check(&json!({ "key": { "$type": true } }), &json!({ "key": 1 })),
        FilterResult::fatal_invalid_filter("$type arg must be a string", "@.key.$type", "@.key")
    );
    assert_eq!(
        CwJsonFilter::check(&json!({ "key": { "$type": "abc" } }), &json!({ "key": 1 })),
        FilterResult::fatal_invalid_filter(
            "$type arg must be a valid type, got `abc`",
            "@.key.$type",
            "@.key"
        )
    );

    // $contains operator
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "$contains": 123 } }),
            &json!({ "key": "abc" })
        ),
        FilterResult::operator_failed(
            "$contains",
            "$contains arg must be a string when applied to a string value",
            "@.key.$contains",
            "@.key"
        )
    );
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "$contains": 1 } }),
            &json!({ "key": true })
        ),
        FilterResult::operator_failed(
            "$contains",
            "value is not a string or an array",
            "@.key.$contains",
            "@.key"
        )
    );

    // $overlap operator
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "$overlap": [1, 2, 3] } }),
            &json!({ "key": "abc" })
        ),
        FilterResult::operator_failed(
            "$overlap",
            "value is not an array",
            "@.key.$overlap",
            "@.key"
        )
    );
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "$overlap": "abc" } }),
            &json!({ "key": "abc" })
        ),
        FilterResult::fatal_invalid_filter(
            "$overlap arg must be an array",
            "@.key.$overlap",
            "@.key"
        )
    );

    // $any operator
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "$any": "abc" } }),
            &json!({ "key": "abc" })
        ),
        FilterResult::operator_failed("$any", "value is not an array", "@.key.$any", "@.key")
    );

    // $all operator
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "$all": "abc" } }),
            &json!({ "key": "abc" })
        ),
        FilterResult::operator_failed("$all", "value is not an array", "@.key.$all", "@.key")
    );

    // $startsWith/$endsWith operator
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "$startsWith": "1" } }),
            &json!({ "key": 123 })
        ),
        FilterResult::operator_failed(
            "$startsWith",
            "value is not a string",
            "@.key.$startsWith",
            "@.key"
        )
    );
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "$endsWith": "3" } }),
            &json!({ "key": 123 })
        ),
        FilterResult::operator_failed(
            "$endsWith",
            "value is not a string",
            "@.key.$endsWith",
            "@.key"
        )
    );
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "$startsWith": 1 } }),
            &json!({ "key": "123" })
        ),
        FilterResult::fatal_invalid_filter(
            "$startsWith arg must be a string",
            "@.key.$startsWith",
            "@.key"
        )
    );
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "$endsWith": 3 } }),
            &json!({ "key": "123" })
        ),
        FilterResult::fatal_invalid_filter(
            "$endsWith arg must be a string",
            "@.key.$endsWith",
            "@.key"
        )
    );

    // #len transformation
    assert_eq!(
        CwJsonFilter::check(&json!({ "key": { "#len": 3 } }), &json!({ "key": 123 })),
        FilterResult::operator_failed(
            "#len",
            "value is not a string, array, or object",
            "@.key.#len",
            "@.key"
        )
    );

    // #to_number transformation
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "#to_number": 3 } }),
            &json!({ "key": true })
        ),
        FilterResult::operator_failed(
            "#to_number",
            "value is not a string or number",
            "@.key.#to_number",
            "@.key"
        )
    );

    // #lower/#upper transformations
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "#lower": "test" } }),
            &json!({ "key": 123 })
        ),
        FilterResult::operator_failed("#lower", "value is not a string", "@.key.#lower", "@.key")
    );
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "#upper": "TEST" } }),
            &json!({ "key": 123 })
        ),
        FilterResult::operator_failed("#upper", "value is not a string", "@.key.#upper", "@.key")
    );

    // #keys transformation
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "#keys": "test" } }),
            &json!({ "key": 123 })
        ),
        FilterResult::operator_failed("#keys", "value is not an object", "@.key.#keys", "@.key")
    );

    // #values transformation
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "#values": "test" } }),
            &json!({ "key": 123 })
        ),
        FilterResult::operator_failed(
            "#values",
            "value is not an object",
            "@.key.#values",
            "@.key"
        )
    );

    // #replace transformation
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "#replace": { "find": 1, "replace": "test", "filter": { "$eq": 1 } } } }),
            &json!({ "key": "test" })
        ),
        FilterResult::fatal_invalid_filter(
            "#replace arg `find` must be a string",
            "@.key.#replace",
            "@.key"
        )
    );
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "#replace": { "find": "test", "replace": 1, "filter": { "$eq": 1 } } } }),
            &json!({ "key": "test" })
        ),
        FilterResult::fatal_invalid_filter(
            "#replace arg `replace` must be a string",
            "@.key.#replace",
            "@.key"
        )
    );
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "#replace": { "find": "test", "replace": "tEsT" } } }),
            &json!({ "key": "test" })
        ),
        FilterResult::fatal_invalid_filter(
            "#replace arg `filter` must be provided",
            "@.key.#replace",
            "@.key"
        )
    );
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "#replace": { "find": "test", "replace": "tEsT", "filter": "atEsT" } } }),
            &json!({ "key": 123 })
        ),
        FilterResult::operator_failed(
            "#replace",
            "value is not a string",
            "@.key.#replace",
            "@.key"
        )
    );
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "#replace": true } }),
            &json!({ "key": "test" })
        ),
        FilterResult::fatal_invalid_filter(
            "#replace arg must be an object",
            "@.key.#replace",
            "@.key"
        )
    );

    // #base64 transformation
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "#base64": 123 } }),
            &json!({ "key": "%" })
        ),
        FilterResult::operator_failed(
            "#base64",
            "failed to decode base64: Invalid symbol 37, offset 0.",
            "@.key.#base64",
            "@.key"
        )
    );
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "#base64": 123 } }),
            &json!({ "key": "test" })
        ),
        FilterResult::operator_failed(
            "#base64",
            "failed to parse decoded base64 value as utf-8 string: invalid utf-8 sequence of 1 bytes from index 0",
            "@.key.#base64",
            "@.key"
        )
    );
    assert_eq!(
        CwJsonFilter::check(
            // "test" -> "dGVzdA=="
            &json!({ "key": { "#base64": "dGVzdA==" } }),
            &json!({ "key": 123 })
        ),
        FilterResult::operator_failed("#base64", "value is not a string", "@.key.#base64", "@.key")
    );

    // #proto transformation
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "#proto": {
                "value": "test"
            } } }),
            &json!({ "key": "test" })
        ),
        FilterResult::fatal_invalid_filter(
            "#proto arg `type` not specified",
            "@.key.#proto",
            "@.key"
        )
    );
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "#proto": {
                "type": "test"
            } } }),
            &json!({ "key": "test" })
        ),
        FilterResult::fatal_invalid_filter(
            "#proto arg `value` not specified",
            "@.key.#proto",
            "@.key"
        )
    );
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "#proto": {
                "type": "test",
                "value": "test"
            } } }),
            &json!({ "key": "test" })
        ),
        FilterResult::fatal_invalid_filter(
            "#proto protobuf decoder not provided",
            "@.key.#proto",
            "@.key"
        )
    );
    assert_eq!(
        CwJsonFilter::new(Some(NoopDecoder))
        .matches(
            &json!({ "key": { "#proto": {
                "type": "test",
                "value": "test"
            } } }),
            &json!({ "key": "%" })
        ),
        FilterResult::operator_failed(
            "#proto",
            "failed to decode protobuf base64 value: Invalid symbol 37, offset 0.",
            "@.key.#proto",
            "@.key"
        )
    );
    assert_eq!(
        CwJsonFilter::new(Some(NoopDecoder))
        .matches(
            &json!({ "key": { "#proto": {
                "type": "test",
                "value": "test"
            } } }),
            &json!({ "key": "test" })
        ),
        FilterResult::operator_failed("#proto", "protobuf decoder not provided", "@.key.#proto", "@.key")
    );
    assert_eq!(
        CwJsonFilter::check(&json!({ "key": { "#proto": {} } }), &json!({ "key": 123 })),
        FilterResult::operator_failed("#proto", "value is not a string", "@.key.#proto", "@.key")
    );
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "#proto": 123 } }),
            &json!({ "key": "test" })
        ),
        FilterResult::fatal_invalid_filter("#proto arg must be an object", "@.key.#proto", "@.key")
    );

    // #stargate transformation
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "#stargate": {
                "value": "test"
            } } }),
            &json!({ "key": "test" })
        ),
        FilterResult::fatal_invalid_filter(
            "#stargate arg `type_url` not specified",
            "@.key.#stargate",
            "@.key"
        )
    );
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "#stargate": {
                "type_url": "test",
                "value": "test"
            } } }),
            &json!({ "key": "test" })
        ),
        FilterResult::fatal_invalid_filter(
            "#stargate arg `type_url` must start with `/`",
            "@.key.#stargate",
            "@.key"
        )
    );
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "#stargate": {
                "type_url": "/test",
            } } }),
            &json!({ "key": "test" })
        ),
        FilterResult::fatal_invalid_filter(
            "#stargate arg `value` not specified",
            "@.key.#stargate",
            "@.key"
        )
    );
    assert_eq!(
        CwJsonFilter::check(
            &json!({ "key": { "#stargate": true } }),
            &json!({ "key": "test" })
        ),
        FilterResult::fatal_invalid_filter(
            "#stargate arg must be an object",
            "@.key.#stargate",
            "@.key"
        )
    );
}
