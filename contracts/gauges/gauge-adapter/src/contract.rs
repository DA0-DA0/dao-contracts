#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coins, from_json, to_json_binary, Addr, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Order, Response, StdError, StdResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::error::ContractError;
use crate::msg::{AdapterQueryMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, ReceiveMsg};
use crate::state::{AssetType, Config, Submission, CONFIG, SUBMISSIONS};

// Version info for migration info.
const CONTRACT_NAME: &str = "crates.io:marketing-gauge-adapter";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let community_pool = deps.api.addr_validate(&msg.community_pool)?;
    SUBMISSIONS.save(
        deps.storage,
        community_pool.clone(),
        &Submission {
            sender: env.contract.address,
            name: "Unimpressed".to_owned(),
            url: "Those funds go back to the community pool".to_owned(),
        },
    )?;

    let config = Config {
        admin: deps.api.addr_validate(&msg.admin)?,
        required_deposit: msg.required_deposit,
        community_pool,
        reward: msg.reward,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20_message(deps, info, msg),
        ExecuteMsg::CreateSubmission { name, url, address } => {
            let received = info.funds.into_iter().next().unwrap_or_default();
            execute::create_submission(
                deps,
                info.sender,
                name,
                url,
                address,
                received.amount,
                AssetType::Native(received.denom),
            )
        }
        ExecuteMsg::ReturnDeposits {} => execute::return_deposits(deps, info.sender),
    }
}

fn receive_cw20_message(
    deps: DepsMut,
    info: MessageInfo,
    msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    match from_json(&msg.msg)? {
        ReceiveMsg::CreateSubmission { name, url, address } => {
            let denom = AssetType::Cw20(info.sender.to_string());
            execute::create_submission(
                deps,
                Addr::unchecked(msg.sender),
                name,
                url,
                address,
                msg.amount,
                denom,
            )
        }
    }
}

fn create_bank_msg(
    denom: &AssetType,
    amount: Uint128,
    recipient: Addr,
) -> Result<CosmosMsg, StdError> {
    match denom {
        AssetType::Cw20(address) => Ok(WasmMsg::Execute {
            contract_addr: address.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: recipient.to_string(),
                amount,
            })?,
            funds: vec![],
        }
        .into()),
        AssetType::Native(denom) => {
            let amount = coins(amount.u128(), denom);
            Ok(BankMsg::Send {
                to_address: recipient.to_string(),
                amount,
            }
            .into())
        }
    }
}

pub mod execute {
    use super::*;

    use cosmwasm_std::{ensure_eq, CosmosMsg};

    pub fn create_submission(
        deps: DepsMut,
        sender: Addr,
        name: String,
        url: String,
        address: String,
        received_amount: Uint128,
        received_denom: AssetType,
    ) -> Result<Response, ContractError> {
        let address = deps.api.addr_validate(&address)?;

        let Config {
            required_deposit,
            community_pool: _,
            reward: _,
            admin: _,
        } = CONFIG.load(deps.storage)?;
        if let Some(required_deposit) = required_deposit {
            if AssetType::Native("".into()) == received_denom {
                return Err(ContractError::DepositRequired {});
            }
            if required_deposit.denom != received_denom {
                return Err(ContractError::InvalidDepositType {});
            }
            if received_amount != required_deposit.amount {
                return Err(ContractError::InvalidDepositAmount {
                    correct_amount: required_deposit.amount,
                });
            }
        } else {
            // If no deposit is required, then any deposit invalidates a submission.
            if !received_amount.is_zero() {
                return Err(ContractError::InvalidDepositAmount {
                    correct_amount: Uint128::zero(),
                });
            }
        }

        // allow to overwrite submission by the same author
        if let Some(old_submission) = SUBMISSIONS.may_load(deps.storage, address.clone())? {
            if old_submission.sender != sender {
                return Err(ContractError::UnauthorizedSubmission {});
            }
        }

        SUBMISSIONS.save(deps.storage, address, &Submission { sender, name, url })?;
        Ok(Response::new().add_attribute("create", "submission"))
    }

    pub fn return_deposits(deps: DepsMut, sender: Addr) -> Result<Response, ContractError> {
        let Config {
            admin,
            required_deposit,
            community_pool: _,
            reward: _,
        } = CONFIG.load(deps.storage)?;

        // No refund if no deposit was required.
        let required_deposit = required_deposit.ok_or(ContractError::NoDepositToRefund {})?;

        ensure_eq!(sender, admin, ContractError::Unauthorized {});

        let msgs = SUBMISSIONS
            .range(deps.storage, None, None, Order::Ascending)
            .map(|item| {
                let (_submission_recipient, submission) = item?;
                create_bank_msg(
                    &required_deposit.denom,
                    required_deposit.amount,
                    submission.sender,
                )
            })
            .collect::<StdResult<Vec<CosmosMsg>>>()?;

        Ok(Response::new().add_messages(msgs))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: AdapterQueryMsg) -> StdResult<Binary> {
    match msg {
        AdapterQueryMsg::Config {} => to_json_binary(&CONFIG.load(deps.storage)?),
        AdapterQueryMsg::AllOptions {} => to_json_binary(&query::all_options(deps)?),
        AdapterQueryMsg::CheckOption { option } => {
            to_json_binary(&query::check_option(deps, option)?)
        }
        AdapterQueryMsg::SampleGaugeMsgs { selected } => {
            to_json_binary(&query::sample_gauge_msgs(deps, selected)?)
        }
        AdapterQueryMsg::Submission { address } => {
            to_json_binary(&query::submission(deps, address)?)
        }
        AdapterQueryMsg::AllSubmissions {} => to_json_binary(&query::all_submissions(deps)?),
    }
}

mod query {
    use cosmwasm_std::{CosmosMsg, Decimal};

    use crate::msg::{
        AllOptionsResponse, AllSubmissionsResponse, CheckOptionResponse, SampleGaugeMsgsResponse,
        SubmissionResponse,
    };

    use super::*;

    pub fn all_options(deps: Deps) -> StdResult<AllOptionsResponse> {
        Ok(AllOptionsResponse {
            options: SUBMISSIONS
                .keys(deps.storage, None, None, Order::Ascending)
                .map(|key| Ok(key?.to_string()))
                .collect::<StdResult<Vec<String>>>()?,
        })
    }

    pub fn check_option(deps: Deps, option: String) -> StdResult<CheckOptionResponse> {
        Ok(CheckOptionResponse {
            valid: SUBMISSIONS.has(deps.storage, deps.api.addr_validate(&option)?),
        })
    }

    pub fn sample_gauge_msgs(
        deps: Deps,
        winners: Vec<(String, Decimal)>,
    ) -> StdResult<SampleGaugeMsgsResponse> {
        let reward = CONFIG.load(deps.storage)?.reward;

        let execute = winners
            .into_iter()
            .map(|(to_address, fraction)| {
                // Gauge already sents chosen tally to this query by using results we send in
                // all_options query; they are already validated
                create_bank_msg(
                    &reward.denom,
                    fraction * reward.amount,
                    Addr::unchecked(to_address),
                )
            })
            .collect::<StdResult<Vec<CosmosMsg>>>()?;
        Ok(SampleGaugeMsgsResponse { execute })
    }

    pub fn submission(deps: Deps, address: String) -> StdResult<SubmissionResponse> {
        let address = deps.api.addr_validate(&address)?;
        let submission = SUBMISSIONS.load(deps.storage, address.clone())?;
        Ok(SubmissionResponse {
            sender: submission.sender,
            name: submission.name,
            url: submission.url,
            address,
        })
    }

    pub fn all_submissions(deps: Deps) -> StdResult<AllSubmissionsResponse> {
        Ok(AllSubmissionsResponse {
            submissions: SUBMISSIONS
                .range(deps.storage, None, None, Order::Ascending)
                .map(|s| {
                    let (address, submission) = s?;
                    Ok(SubmissionResponse {
                        sender: submission.sender,
                        name: submission.name,
                        url: submission.url,
                        address,
                    })
                })
                .collect::<StdResult<Vec<SubmissionResponse>>>()?,
        })
    }
}

/// Manages the contract migration.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Decimal, Uint128,
    };

    use crate::state::Asset;

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            admin: "admin".to_owned(),
            required_deposit: Some(Asset::new_cw20("wynd", 10_000_000)),
            community_pool: "community".to_owned(),
            reward: Asset::new_native("ujuno", 150_000_000_000),
        };
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("user", &[]),
            msg.clone(),
        )
        .unwrap();

        // Check if the config is stored.
        let config = CONFIG.load(deps.as_ref().storage).unwrap();
        assert_eq!(config.admin, Addr::unchecked("admin"));
        assert_eq!(
            config.required_deposit,
            Some(Asset {
                denom: AssetType::Cw20("wynd".to_owned()),
                amount: Uint128::new(10_000_000)
            })
        );
        assert_eq!(config.community_pool, "community".to_owned());
        assert_eq!(
            config.reward,
            Asset {
                denom: AssetType::Native("ujuno".to_owned()),
                amount: Uint128::new(150_000_000_000)
            }
        );

        let msg = InstantiateMsg {
            reward: Asset::new_native("ujuno", 10_000_000),
            ..msg
        };
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("user", &[]),
            msg.clone(),
        )
        .unwrap();
        let config = CONFIG.load(deps.as_ref().storage).unwrap();
        assert_eq!(
            config.reward,
            Asset {
                denom: AssetType::Native("ujuno".to_owned()),
                amount: Uint128::new(10_000_000)
            }
        );

        let msg = InstantiateMsg {
            required_deposit: None,
            ..msg
        };
        instantiate(deps.as_mut(), mock_env(), mock_info("user", &[]), msg).unwrap();
        let config = CONFIG.load(deps.as_ref().storage).unwrap();
        assert_eq!(config.required_deposit, None);
    }

    #[test]
    fn sample_gauge_msgs_native() {
        let mut deps = mock_dependencies();

        let reward = Uint128::new(150_000_000_000);
        let msg = InstantiateMsg {
            admin: "admin".to_owned(),
            required_deposit: Some(Asset::new_cw20("wynd", 10_000_000)),
            community_pool: "community".to_owned(),
            reward: Asset::new_native("ujuno", reward.into()),
        };
        instantiate(deps.as_mut(), mock_env(), mock_info("user", &[]), msg).unwrap();

        let selected = vec![
            (
                "juno1t8ehvswxjfn3ejzkjtntcyrqwvmvuknzy3ajxy".to_string(),
                Decimal::percent(41),
            ),
            (
                "juno196ax4vc0lwpxndu9dyhvca7jhxp70rmcl99tyh".to_string(),
                Decimal::percent(33),
            ),
            (
                "juno1y0us8xvsvfvqkk9c6nt5cfyu5au5tww23dmh40".to_string(),
                Decimal::percent(26),
            ),
        ];
        let res = query::sample_gauge_msgs(deps.as_ref(), selected).unwrap();
        assert_eq!(res.execute.len(), 3);
        assert_eq!(
            res.execute,
            [
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: "juno1t8ehvswxjfn3ejzkjtntcyrqwvmvuknzy3ajxy".to_string(),
                    amount: coins((reward * Decimal::percent(41)).u128(), "ujuno")
                }),
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: "juno196ax4vc0lwpxndu9dyhvca7jhxp70rmcl99tyh".to_string(),
                    amount: coins((reward * Decimal::percent(33)).u128(), "ujuno")
                }),
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: "juno1y0us8xvsvfvqkk9c6nt5cfyu5au5tww23dmh40".to_string(),
                    amount: coins((reward * Decimal::percent(26)).u128(), "ujuno")
                }),
            ]
        );
    }

    #[test]
    fn sample_gauge_msgs_cw20() {
        let mut deps = mock_dependencies();

        let reward = Uint128::new(150_000_000_000);
        let msg = InstantiateMsg {
            admin: "admin".to_owned(),
            required_deposit: Some(Asset::new_cw20("wynd", 10_000_000)),
            community_pool: "community".to_owned(),
            reward: Asset::new_cw20("wynd", reward.into()),
        };
        instantiate(deps.as_mut(), mock_env(), mock_info("user", &[]), msg).unwrap();

        let selected = vec![
            (
                "juno1t8ehvswxjfn3ejzkjtntcyrqwvmvuknzy3ajxy".to_string(),
                Decimal::percent(41),
            ),
            (
                "juno196ax4vc0lwpxndu9dyhvca7jhxp70rmcl99tyh".to_string(),
                Decimal::percent(33),
            ),
            (
                "juno1y0us8xvsvfvqkk9c6nt5cfyu5au5tww23dmh40".to_string(),
                Decimal::percent(26),
            ),
        ];
        let res = query::sample_gauge_msgs(deps.as_ref(), selected).unwrap();
        assert_eq!(res.execute.len(), 3);
        assert_eq!(
            res.execute,
            [
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "wynd".to_owned(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: "juno1t8ehvswxjfn3ejzkjtntcyrqwvmvuknzy3ajxy".to_string(),
                        amount: reward * Decimal::percent(41)
                    })
                    .unwrap(),
                    funds: vec![]
                }),
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "wynd".to_owned(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: "juno196ax4vc0lwpxndu9dyhvca7jhxp70rmcl99tyh".to_string(),
                        amount: reward * Decimal::percent(33)
                    })
                    .unwrap(),
                    funds: vec![]
                }),
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "wynd".to_owned(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: "juno1y0us8xvsvfvqkk9c6nt5cfyu5au5tww23dmh40".to_string(),
                        amount: reward * Decimal::percent(26)
                    })
                    .unwrap(),
                    funds: vec![]
                }),
            ]
        );
    }

    #[test]
    fn return_deposits_authorization() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            admin: "admin".to_owned(),
            required_deposit: None,
            community_pool: "community".to_owned(),
            reward: Asset::new_native("ujuno", 150_000_000_000),
        };
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("user", &[]),
            msg.clone(),
        )
        .unwrap();

        let err = execute::return_deposits(deps.as_mut(), Addr::unchecked("user")).unwrap_err();
        assert_eq!(err, ContractError::NoDepositToRefund {});

        let msg = InstantiateMsg {
            required_deposit: Some(Asset::new_native("ujuno", 10_000_000)),
            ..msg
        };
        instantiate(deps.as_mut(), mock_env(), mock_info("user", &[]), msg).unwrap();

        let err = execute::return_deposits(deps.as_mut(), Addr::unchecked("user")).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});
    }
}
