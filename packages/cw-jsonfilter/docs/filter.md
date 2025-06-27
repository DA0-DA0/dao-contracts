# Filter Syntax

Every filter is a JSON object. At its simplest form, a filter acts as a mask,
checking if the values of the filter match those of the object being filtered.

```json
{
  "key": "value",
  "nested": {
    "key": "value"
  }
}
```

This filter would match against an object like:

```json
{
  "key": "value",
  "other": "data",
  "nested": {
    "key": "value"
  }
}
```

## Advanced Filtering with Operators

Operators enable complex filtering by allowing operations on keys and values.

### **Example**: Logical Chaining and Regex

```json
{
  "$or": [{ "key": { "$regex": "^L" } }, { "key": { "$regex": "in$" } }]
}
```

This filter would match all of these objects:

```json
{ "key": "Login" }
{ "key": "Lenin" }
```

but not:

```json
{ "key": "lol" }
```

## Logical Operators

### `$and`

Chain multiple filters together. All must evaluate to `true`.

```json
{
  "$and": [{ "key": "val" }, { "another": "filter" }]
}
```

This is also implied by adjacent keys:

```json
{
  "number": {
    "$gt": 5,
    "$lt": 10
  }
}
```

is the same as:

```json
{
  "$and": [{ "number": { "$gt": 5 } }, { "number": { "$lt": 10 } }]
}
```

and:

```json
{
  "number": {
    "$and": [{ "$gt": 5 }, { "$lt": 10 }]
  }
}
```

### `$or`

Chain multiple filters together. At least one must evaluate to `true`.

```json
{
  "$or": [{ "key": "val" }, { "another": "filter" }]
}
```

### `$nor`

Chain multiple filters together. None must evaluate to `true` (opposite of
`$or`).

```json
{
  "$nor": [{ "status": "banned" }, { "status": "suspended" }]
}
```

### `$xor`

Chain multiple filters together. Exactly one must evaluate to `true`.

```json
{
  "$xor": [{ "is_premium": true }, { "is_trial": true }]
}
```

### `$not`

Inverts the result of the nested filter expression.

```json
{ "$not": { "key": "value" } }
```

## Comparison Operators

### `$eq`

Explicitly checks for equality (implicit in basic filters).

```json
{ "key": { "$eq": "value" } }
```

is the same as:

```json
{ "key": "value" }
```

### `$ne` & `$neq`

Evaluates to `true` if the value is not equal to the specified value.

```json
{ "key": { "$ne": "value" } }
{ "key": { "$neq": "value" } }
```

is the same as:

```json
{ "key": { "$not": { "$eq": "value" } } }
{ "key": { "$not": "value" } }
```

### `$lt` & `$lte`

Evaluates to `true` if the value is less than or equal to the specified value.

```json
{ "age": { "$lt": 18 } }
{ "age": { "$lte": 65 } }
```

### `$gt` & `$gte`

Evaluates to `true` if the value is greater than or equal to the specified
value.

```json
{ "score": { "$gt": 85 } }
{ "age": { "$gte": 18 } }
```

### `$range` & `$between`

Evaluates to `true` if the value is within the specified range (inclusive).

```json
{ "age": { "$range": [18, 65] } }
{ "score": { "$between": [70, 100] } }
```

### `$range_exclusive` & `$between_exclusive`

Evaluates to `true` if the value is within the specified range (exclusive).

```json
{ "temperature": { "$range_exclusive": [0, 100] } }
{ "percentage": { "$between_exclusive": [0, 1] } }
```

### `$type`

Evaluates to `true` if the value matches the specified type.

```json
{ "count": { "$type": "number" } }
{ "name": { "$type": "string" } }
{ "tags": { "$type": "array" } }
{ "metadata": { "$type": "object" } }
{ "flag": { "$type": "boolean" } }
{ "empty": { "$type": "null" } }
```

## Existence Operators

### `$exists`

Checks whether the key exists in the object.

```json
{ "email": { "$exists": true } }
{ "optional_field": { "$exists": false } }
```

## Array/String/Object Operators

### `$in` & `$contains`

For arrays: Evaluates to `true` if the array contains the specified value.

For strings: Evaluates to `true` if the string contains the specified substring.

```json
{ "tags": { "$contains": "programming" } }
{ "description": { "$contains": "rust" } }
{ "numbers": { "$in": 42 } }
```

### `$empty`

Evaluates to `true` if the array, object, or string is empty.

```json
{ "tags": { "$empty": true } }
{ "description": { "$empty": false } }
{ "metadata": { "$empty": true } }
```

### `$overlap`

Evaluates to `true` if arrays have at least one common element.

```json
{ "user_roles": { "$overlap": ["admin", "moderator"] } }
```

### `$any`

Evaluates to `true` if any array element matches the filter.

```json
{ "scores": { "$any": { "$gt": 90 } } }
{ "users": { "$any": { "role": "admin" } } }
```

### `$all`

Evaluates to `true` if all array elements match the filter.

```json
{ "scores": { "$all": { "$gte": 60 } } }
{ "items": { "$all": { "status": "active" } } }
```

## String Operators

### `$regex` & `$match`

Evaluates to `true` if the value matches the regular expression pattern.

```json
{ "email": { "$regex": "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$" } }
{ "phone": { "$match": "^\\+?[1-9]\\d{1,14}$" } }
```

### `$startsWith`

Evaluates to `true` if the string starts with the specified prefix.

```json
{ "name": { "$startsWith": "Dr." } }
{ "url": { "$startsWith": "https://" } }
```

### `$endsWith`

Evaluates to `true` if the string ends with the specified suffix.

```json
{ "filename": { "$endsWith": ".pdf" } }
{ "email": { "$endsWith": "@company.com" } }
```

## Value Transformations

Value transformations allow you to modify a value before applying a filter to
it. They use the `#` prefix.

### `#len` & `#size`

Get the length/size of a string or array, then apply the filter to that number.

```json
{ "password": { "#len": { "$gte": 8 } } }
{ "items": { "#size": { "$eq": 0 } } }
{ "tags": { "#len": { "$range": [1, 5] } } }
```

### `#lower`

Convert string to lowercase, then apply the filter.

```json
{ "name": { "#lower": { "$eq": "john doe" } } }
{ "email": { "#lower": { "$endsWith": "@gmail.com" } } }
```

### `#upper`

Convert string to uppercase, then apply the filter.

```json
{ "code": { "#upper": { "$startsWith": "US" } } }
{ "country": { "#upper": { "$eq": "UNITED STATES" } } }
```

### `#keys`

Get object keys as an array, then apply the filter.

```json
{ "metadata": { "#keys": { "$contains": "version" } } }
{ "config": { "#keys": { "#len": { "$gt": 0 } } } }
```

### `#values`

Get object values as an array, then apply the filter.

```json
{ "scores": { "#values": { "$any": { "$gt": 95 } } } }
{ "settings": { "#values": { "$all": { "$type": "string" } } } }
```

### `#base64`

Decode a base64 string and apply the filter to the decoded content. If the
decoded content is valid JSON, it will be parsed; otherwise, it's treated as a
string.

```json
{ "encoded_data": { "#base64": { "name": "John" } } }
{ "token": { "#base64": { "#contains": "user_id" } } }
```

### `#proto`

Decodes a binary string with a protobuf type and applies the filter to the
decoded value. Specify the type of the protobuf value in the `type` field, and
the value filter to apply to the decoded value in the `value` field.

```json
{
  "proto_data": {
    "#proto": {
      "type": "google.protobuf.StringValue",
      "value": "John"
    }
  }
}
{
  "protobuf_field": {
    "#proto": {
      "type": "cosmos.bank.v1beta1.MsgSend",
      "value": {"amount": [{"denom": "uatom", "amount": "1000"}]}
    }
  }
}
```

#### Registering Protobuf Types

See [protobuf.md](protobuf.md) for more information on how to register protobuf
types.

## Complex Examples

### Nested Filters with Multiple Operators

```json
{
  "$and": [
    {
      "$or": [{ "age": { "$gte": 18 } }, { "is_student": true }]
    },
    {
      "$not": {
        "$or": [{ "city": "New York" }, { "city": "Los Angeles" }]
      }
    }
  ]
}
```

### Array Element Filtering

```json
{
  "users": {
    "$any": {
      "$and": [{ "role": "admin" }, { "last_login": { "$gte": "2024-01-01" } }]
    }
  }
}
```

### String Transformations with Regex

```json
{
  "email": {
    "#lower": {
      "$regex": "^[a-z0-9._%+-]+@company\\.com$"
    }
  }
}
```

### Complex Validation

```json
{
  "$and": [
    { "age": { "$type": "number" } },
    { "age": { "$range": [13, 120] } },
    { "name": { "$type": "string" } },
    { "name": { "#len": { "$range": [1, 100] } } },
    {
      "email": {
        "#lower": { "$regex": "^[a-z0-9._%+-]+@[a-z0-9.-]+\\.[a-z]{2,}$" }
      }
    }
  ]
}
```

### Working with Encoded Data

```json
{
  "jwt_payload": {
    "#base64": {
      "$and": [
        { "user_id": { "$type": "string" } },
        { "exp": { "$gt": 1672531200 } }
      ]
    }
  }
}
```
