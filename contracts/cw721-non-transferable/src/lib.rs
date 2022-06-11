use cosmwasm_std::Empty;
pub use cw721_base::{ContractError, InstantiateMsg, ExecuteMsg, MintMsg, MinterResponse, QueryMsg};
use cw721::{Cw721ReceiveMsg};

pub type Cw721NonTransferableContract<'a> = cw721_base::Cw721Contract<'a, Empty, Empty>;

#[cfg(not(feature = "library"))]
pub mod entry {
    use super::*;

    use cosmwasm_std::entry_point;
    use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

    #[entry_point]
    pub fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> StdResult<Response> {
        Cw721NonTransferableContract::default().instantiate(deps, env, info, msg)
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg<Empty>,
    ) -> Result<Response, ContractError> {
        let minter = Cw721NonTransferableContract::default().minter.load(deps.storage)?;

        if info.sender != minter {
            return Err(ContractError::Unauthorized {});
        }

        match msg {
            ExecuteMsg::Mint(msg) => mint(deps, env, info, msg),
            ExecuteMsg::TransferNft {
                recipient,
                token_id,
            } => transfer_nft(deps, env, info, recipient, token_id),
            ExecuteMsg::SendNft {
                contract,
                token_id,
                msg,
            } => send_nft(deps, env, info, contract, token_id, msg),
            ExecuteMsg::Burn { token_id } => burn(deps, env, info, token_id),
            _ => Err(ContractError::Unauthorized {}),
        }
    }

    fn mint(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: MintMsg<Empty>,
    ) -> Result<Response, ContractError> {
        Cw721NonTransferableContract::default().mint(deps, env, info, msg)
    }

    fn transfer_nft(
        deps: DepsMut,
        env: Env,
        _info: MessageInfo,
        recipient: String,
        token_id: String,
    ) -> Result<Response, ContractError> {
        let token = Cw721NonTransferableContract::default().tokens.load(deps.storage, &token_id)?;

        let internal_info = MessageInfo {
            sender: token.owner,
            funds: vec![],
        };
        //println!("{}",token.owner);
        Cw721NonTransferableContract::default()._transfer_nft(deps, &env, &internal_info, &recipient, &token_id)?;

        Ok(Response::new()
            .add_attribute("action", "transfer_nft")
            .add_attribute("sender", internal_info.sender)
            .add_attribute("recipient", recipient)
            .add_attribute("token_id", token_id))
    }

    fn send_nft(
        deps: DepsMut,
        env: Env,
        _info: MessageInfo,
        contract: String,
        token_id: String,
        msg: Binary,
    ) -> Result<Response, ContractError> {
        let token = Cw721NonTransferableContract::default().tokens.load(deps.storage, &token_id)?;

        let internal_info = MessageInfo {
            sender: token.owner,
            funds: vec![],
        };

        // Transfer token
        Cw721NonTransferableContract::default()._transfer_nft(deps, &env, &internal_info, &contract, &token_id)?;

        let send = Cw721ReceiveMsg {
            sender: internal_info.sender.to_string(),
            token_id: token_id.clone(),
            msg,
        };

        // Send message
        Ok(Response::new()
            .add_message(send.into_cosmos_msg(contract.clone())?)
            .add_attribute("action", "send_nft")
            .add_attribute("sender", internal_info.sender)
            .add_attribute("recipient", contract)
            .add_attribute("token_id", token_id))
    }

    fn burn(
        deps: DepsMut,
        env: Env,
        _info: MessageInfo,
        token_id: String,
    ) -> Result<Response, ContractError> {
        let token = Cw721NonTransferableContract::default().tokens.load(deps.storage, &token_id)?;

        let internal_info = MessageInfo {
            sender: token.owner.clone(),
            funds: vec![],
        };
        
        Cw721NonTransferableContract::default().check_can_send(deps.as_ref(), &env, &internal_info, &token)?;

        Cw721NonTransferableContract::default().tokens.remove(deps.storage, &token_id)?;
        Cw721NonTransferableContract::default().decrement_tokens(deps.storage)?;

        Ok(Response::new()
            .add_attribute("action", "burn")
            .add_attribute("sender", internal_info.sender)
            .add_attribute("token_id", token_id))

    }


    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        Cw721NonTransferableContract::default().query(deps, env, msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cw721::Cw721Query;

    const CREATOR: &str = "creator";
    const MEMBER1: &str = "member1";
    const MEMBER2: &str = "member2";

    #[test]
    fn use_metadata_extension() {
        let mut deps = mock_dependencies();
        let contract = Cw721NonTransferableContract::default();

        let info = mock_info(CREATOR, &[]);
        let init_msg = InstantiateMsg {
            name: "SpaceShips".to_string(),
            symbol: "SPACE".to_string(),
            minter: CREATOR.to_string(),
        };
        contract
            .instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg)
            .unwrap();

        let token_id = "Enterprise";
        let mint_msg = MintMsg {
            token_id: token_id.to_string(),
            owner: "john".to_string(),
            token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
            extension: Empty {},
        };
        let exec_msg = ExecuteMsg::Mint(mint_msg.clone());
        contract
            .execute(deps.as_mut(), mock_env(), info, exec_msg)
            .unwrap();

        let res = contract.nft_info(deps.as_ref(), token_id.into()).unwrap();
        assert_eq!(res.token_uri, mint_msg.token_uri);
        assert_eq!(res.extension, mint_msg.extension);
    }

    #[test]
    fn verify_token_owner_cannot_transfer() {
        let mut deps = mock_dependencies();
        let contract = Cw721NonTransferableContract::default();

        let info = mock_info(CREATOR, &[]);
        let init_msg = InstantiateMsg {
            name: "SpaceShips".to_string(),
            symbol: "SPACE".to_string(),
            minter: CREATOR.to_string(),
        };
        contract
            .instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg)
            .unwrap();

        let token_id = "Enterprise";
        let mint_msg = MintMsg {
            token_id: token_id.to_string(),
            owner: MEMBER1.to_string(),
            token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
            extension: Empty {},
        };
        let exec_msg = ExecuteMsg::Mint(mint_msg.clone());
        contract
            .execute(deps.as_mut(), mock_env(), info, exec_msg)
            .unwrap();

        let info = mock_info(MEMBER1, &[]);
        
        let transfer_msg = ExecuteMsg::TransferNft {
            recipient: MEMBER2.into(),
            token_id: token_id.to_string(),
        };

        contract
            .execute(deps.as_mut(), mock_env(), info, transfer_msg)
            .unwrap_err();
    }

    #[test]
    fn verify_token_minter_can_transfer() {
        let mut deps = mock_dependencies();
        let contract = Cw721NonTransferableContract::default();

        let info = mock_info(CREATOR, &[]);
        let init_msg = InstantiateMsg {
            name: "SpaceShips".to_string(),
            symbol: "SPACE".to_string(),
            minter: CREATOR.to_string(),
        };
        contract
            .instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg)
            .unwrap();

        let token_id = "Enterprise";
        let mint_msg = MintMsg {
            token_id: token_id.to_string(),
            owner: MEMBER1.to_string(),
            token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
            extension: Empty {},
        };
        let exec_msg = ExecuteMsg::Mint(mint_msg.clone());
        contract
            .execute(deps.as_mut(), mock_env(), info.clone(), exec_msg)
            .unwrap();

        let transfer_msg = ExecuteMsg::TransferNft {
            recipient: MEMBER2.into(),
            token_id: token_id.to_string(),
        };
        println!("{}", token_id);
        contract
            .execute(deps.as_mut(), mock_env(), info, transfer_msg)
            .unwrap();
    }
}
