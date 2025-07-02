#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::{base64_encode_protobuf, get_protobuf_messages, CwJsonFilter};
    use prost_reflect::{prost::Message, prost_types::FileDescriptorSet};
    use serde_json::json;

    #[test]
    fn array_match() {
        assert!(
            CwJsonFilter::check(&json!({ "a": [1, 2, 3, 4]}), &json!({ "a": [1, 2, 3, 4]}))
                .is_pass()
        );

        assert!(
            CwJsonFilter::check(&json!({ "a": [1, 2, 3, 4]}), &json!({ "a": [1, 2, 3]})).is_fail()
        );

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
            &json!({ "key": 4})
        )
        .is_fail());
        assert!(CwJsonFilter::check(
            &json!({
                "key": {
                    "$gt": 5
                }
            }),
            &json!({ "key": 5})
        )
        .is_fail());
        assert!(CwJsonFilter::check(
            &json!({
                "key": {
                    "$gte": 5
                }
            }),
            &json!({ "key": 5})
        )
        .is_pass());
    }

    #[test]
    fn less_than() {
        assert!(CwJsonFilter::check(
            &json!({
                "key": {
                    "$lt": 5
                }
            }),
            &json!({ "key": 6})
        )
        .is_fail());
        assert!(CwJsonFilter::check(
            &json!({
                "key": {
                    "$lt": 5
                }
            }),
            &json!({ "key": 5})
        )
        .is_fail());
        assert!(CwJsonFilter::check(
            &json!({
                "key": {
                    "$lte": 5
                }
            }),
            &json!({ "key": 5})
        )
        .is_pass());
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
    fn in_array() {
        assert!(CwJsonFilter::check(
            &json!({
                "key": {
                    "$in": 3
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
                        "$in": 3
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
                "list": { "#len": 3}
            }),
            &json!({
                "list": [1,2,3]
            })
        )
        .is_pass());
        assert!(CwJsonFilter::check(
            &json!({
                "list": { "#size": 5}
            }),
            &json!({
                "list": [1,2,3]
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
    fn regex_match() {
        assert!(CwJsonFilter::check(
            &json!({
                "key": { "$regex": "hello (world|json)"}
            }),
            &json!({
                "key": "hello world"
            })
        )
        .is_pass());
        assert!(CwJsonFilter::check(
            &json!({
                "key": { "$regex": "hello (world|json)"}
            }),
            &json!({
                "key": "hello json"
            })
        )
        .is_pass());
        assert!(CwJsonFilter::check(
            &json!({
                "key": { "$regex": "hello (world|json)"}
            }),
            &json!({
                "key": "hello rust"
            })
        )
        .is_fail());
        assert!(CwJsonFilter::check(
            &json!({
                "key": { "$regex": "hello (world|json)"}
            }),
            &json!({
                "key": "hello world!"
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
            "$nor": [
                { "status": "banned" },
                { "status": "suspended" }
            ]
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
        assert!(
            CwJsonFilter::check(&filter, &json!({"is_premium": true, "is_trial": true})).is_fail()
        );
        assert!(
            CwJsonFilter::check(&filter, &json!({"is_premium": false, "is_trial": false}))
                .is_fail()
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
    fn empty_op() {
        // Array
        assert!(
            CwJsonFilter::check(&json!({"tags": {"$empty": true}}), &json!({"tags": []})).is_pass()
        );

        assert!(CwJsonFilter::check(
            &json!({"tags": {"$empty": false}}),
            &json!({"tags": ["tag1"]})
        )
        .is_pass());

        assert!(CwJsonFilter::check(
            &json!({"tags": {"$empty": true}}),
            &json!({"tags": ["tag1"]})
        )
        .is_fail());

        // String
        assert!(CwJsonFilter::check(
            &json!({"description": {"$empty": true}}),
            &json!({"description": ""})
        )
        .is_pass());

        assert!(CwJsonFilter::check(
            &json!({"description": {"$empty": false}}),
            &json!({"description": "hello"})
        )
        .is_pass());

        // Object
        assert!(CwJsonFilter::check(
            &json!({"metadata": {"$empty": true}}),
            &json!({"metadata": {}})
        )
        .is_pass());

        assert!(CwJsonFilter::check(
            &json!({"metadata": {"$empty": false}}),
            &json!({"metadata": {"key": "value"}})
        )
        .is_pass());

        assert!(
            CwJsonFilter::check(&json!({"value": {"$empty": true}}), &json!({"value": null}))
                .is_fail()
        );
    }

    #[test]
    fn overlap_op() {
        let filter = json!({"user_roles": {"$overlap": ["admin", "moderator"]}});

        assert!(CwJsonFilter::check(&filter, &json!({"user_roles": ["admin", "user"]})).is_pass());
        assert!(CwJsonFilter::check(&filter, &json!({"user_roles": ["moderator"]})).is_pass());
        assert!(
            CwJsonFilter::check(&filter, &json!({"user_roles": ["admin", "moderator"]})).is_pass()
        );

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
    fn match_op() {
        assert!(CwJsonFilter::check(
            &json!({"phone": {"$match": "^\\+?[1-9]\\d{1,14}$"}}),
            &json!({"phone": "+1234567890"})
        )
        .is_pass());

        assert!(CwJsonFilter::check(
            &json!({"phone": {"$match": "^\\+?[1-9]\\d{1,14}$"}}),
            &json!({"phone": "1234567890"})
        )
        .is_pass());

        assert!(CwJsonFilter::check(
            &json!({"phone": {"$match": "^\\+?[1-9]\\d{1,14}$"}}),
            &json!({"phone": "invalid-phone"})
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
    fn string_transformation_with_regex() {
        let filter = json!({
            "email": {
                "#lower": {
                    "$regex": "^[a-z0-9._%+-]+@company\\.com$"
                }
            }
        });

        assert!(CwJsonFilter::check(
            &filter,
            &json!({
                "email": "USER@Company.Com"
            })
        )
        .is_pass());

        assert!(CwJsonFilter::check(
            &filter,
            &json!({
                "email": "user@gmail.com"
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
                {"email": {"#lower": {"$regex": "^[a-z0-9._%+-]+@[a-z0-9.-]+\\.[a-z]{2,}$"}}}
            ]
        });

        assert!(CwJsonFilter::check(
            &filter,
            &json!({
                "age": 25,
                "name": "John Doe",
                "email": "john.doe@Example.Com"
            })
        )
        .is_pass());

        // Invalid age
        assert!(CwJsonFilter::check(
            &filter,
            &json!({
                "age": 150,
                "name": "John Doe",
                "email": "john.doe@example.com"
            })
        )
        .is_fail());

        // Invalid email format
        assert!(CwJsonFilter::check(
            &filter,
            &json!({
                "age": 25,
                "name": "John Doe",
                "email": "invalid-email"
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

        assert!(
            CwJsonFilter::check(&json!({"items": {"$and": []}}), &json!({"items": {}})).is_pass()
        ); // $and passes on empty arrays

        assert!(
            CwJsonFilter::check(&json!({"items": {"$or": []}}), &json!({"items": {}})).is_fail()
        ); // $or fails on empty arrays

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
    }

    #[test]
    fn protobuf_filter() {
        // Use CARGO_MANIFEST_DIR to get the crate root reliably
        let crate_root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        let proto_path = std::path::Path::new(&crate_root).join("proto/string_bool_value.pb");

        let file_descriptor_sets =
            vec![FileDescriptorSet::decode(std::fs::read(proto_path).unwrap().as_slice()).unwrap()];

        let cwjf = CwJsonFilter::new(file_descriptor_sets);
        let pool = cwjf.pool.clone().unwrap();

        // String filter

        let string_filter = json!({"someProto": {"#proto": {"type": "google.protobuf.StringValue", "value": "pass"}}});
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
        let filter = json!({"someProto": {"#proto": {"type": "google.protobuf.StringValue", "value": "pass"}}});
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
        let filter = json!({"duration": {"#replace": {"pattern": "^(\\d+)s$", "replacement": "$1", "filter": {"#to_number": {"$gt": 0}}}}});
        assert!(CwJsonFilter::check(&filter, &json!({"duration": "1000s"})).is_pass());
        assert!(CwJsonFilter::check(&filter, &json!({"duration": "1000"})).is_pass());
        assert!(CwJsonFilter::check(&filter, &json!({"duration": "1000ms"})).is_fail());
    }
}
