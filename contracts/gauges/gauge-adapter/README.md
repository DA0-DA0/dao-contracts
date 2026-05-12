# Marketing Gauge Adapter

An example adapter for the [gauge orchestrator](../gauge/README.md). It
implements a project-registry-with-bond pattern: projects apply by posting
a refundable deposit, the gauge's voters allocate the reward proportionally
to applicants, and the orchestrator dispatches the payouts at each epoch.

See [`contracts/gauges/README.md`](../README.md) for the broader two-contract
design (orchestrator + adapter); this README focuses on the adapter's
specific semantics. Other adapters can plug into the same orchestrator by
implementing the three `AdapterQueryMsg` variants.

## Lifecycle

1. **Instantiate.** The DAO uploads this contract and instantiates with:
   - `admin` — the only address allowed to call `ReturnDeposits`.
   - `required_deposit` — optional native or cw20 bond per submission.
   - `community_pool` — refund target for unbid funds (gets a default
     "Unimpressed" submission so unused weight isn't lost).
   - `reward` — total payout pool (native or cw20) for the gauge's epoch.

2. **Submit.** Projects call `ExecuteMsg::CreateSubmission { name, url,
   address }`. If `required_deposit` is native they attach the deposit as
   `funds`; if it's a cw20 they `Send` the cw20 to this contract with an
   inner `ReceiveMsg::CreateSubmission`. Submissions are keyed by
   `address` (the *recipient* of any future reward, distinct from the
   submitter). Re-submitting from the same sender overwrites; submitting
   to an address already claimed by a different sender fails with
   `UnauthorizedSubmission`.

3. **Vote.** Once the gauge orchestrator points at this adapter, voters
   weight the submitted addresses. The orchestrator queries `AllOptions`
   for the option set and `CheckOption` for user-added options.

4. **Execute.** At epoch close, the orchestrator queries
   `SampleGaugeMsgs { selected }` (where `selected` is `Vec<(address,
   Decimal)>` with weights summing to ≤1.0). The adapter returns
   `Vec<CosmosMsg>` — one transfer per recipient, native or cw20 depending
   on how `reward` was configured.

5. **Refund.** The admin calls `ReturnDeposits` to refund all posted
   deposits in one shot.

## ExecuteMsg

| Variant | Auth | Notes |
|---|---|---|
| `CreateSubmission { name, url, address }` | anyone | Native-deposit path. Funds must match `required_deposit` exactly (or be empty if no deposit required). |
| `Receive(Cw20ReceiveMsg)` | the configured cw20 | cw20-deposit path. Sender of the cw20 `Send` becomes the submission's sender. |
| `ReturnDeposits {}` | `admin` | Refunds every posted deposit to its sender. |

## QueryMsg (`AdapterQueryMsg`)

| Variant | Response | Purpose |
|---|---|---|
| `Config {}` | `Config` | Inspect the deployed parameters. |
| `AllOptions {}` | `AllOptionsResponse` | Used by the orchestrator on gauge attach. |
| `CheckOption { option }` | `CheckOptionResponse { valid: bool }` | Used by the orchestrator when a voter calls `AddOption`. |
| `SampleGaugeMsgs { selected }` | `SampleGaugeMsgsResponse { execute: Vec<CosmosMsg> }` | Translates a selected set into payout messages. |
| `Submission { address }` | `SubmissionResponse` | Read a single submission. |
| `AllSubmissions {}` | `AllSubmissionsResponse` | List all submissions. |

## Errors

| Variant | Trigger |
|---|---|
| `Unauthorized` | Non-admin called `ReturnDeposits`. |
| `UnauthorizedSubmission` | Submission to a recipient claimed by a different sender. |
| `InvalidDepositType` | Sent the wrong denom / wrong cw20. |
| `InvalidDepositAmount { correct_amount }` | Sent the right denom but wrong amount. |
| `NoDepositToRefund` | `ReturnDeposits` called on a deposit-less adapter. |
| `PaymentError` | Missing funds when a deposit is required. |

## Writing a different adapter

Any contract that answers `AllOptions`, `CheckOption`, and
`SampleGaugeMsgs` can plug into the orchestrator. Common patterns:

- **Validator-delegation adapter.** Options = validator operator
  addresses; `SampleGaugeMsgs` emits `MsgDelegate` / `MsgRedelegate` for
  the DAO's staking position.
- **AMM-incentive adapter.** Options = pool IDs; `SampleGaugeMsgs` emits
  the AMM-specific incentive-funding message proportional to weights.
- **Budget-allocation adapter.** Options = bank addresses;
  `SampleGaugeMsgs` emits `BankMsg::Send` proportional to weights.

The marketing adapter shipped here is one shape (registry + bond +
proportional payout). The orchestrator is agnostic to the choice.
