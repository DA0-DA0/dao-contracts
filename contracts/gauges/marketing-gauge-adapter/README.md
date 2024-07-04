# Marketing Gauge Adapter Contract

This is an adapter contract for use in conjunction with the [WYND gauge contract](https://github.com/cosmorama/wynddao/tree/main/contracts/gauge). The purpose of this adapter is to allow people related to marketing to apply for a reward. The total reward amount is set during contract instantiation and will be divided among applicants based on community votes.

## Implementation

The basic structure containing all information required for an application is:

```rust
CreateSubmission {
    name: String,
    url: String,
    address: String,
```

Depending on how the gauge contract is instantiated a spam preventing deposit can be required to create a submission. This is specified by the field `required_deposit` in the `Config` structure.

The contract can receive 3 kind of messages to execute the contract logic:

1. Create a submission by sending CW20 tokens routed through the CW20 contract.

2. Create a submission by sending native tokens.

3. Return all submission's deposit to the address specified during submission. This logic can be triggered only by the contract's `admin` specified during gauge instantiation and saved in the `Config` structure.

## Options

Options represent all the addresses that have been stored through the field `address` of the `CreateSubmission` structure.
