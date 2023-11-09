use cosmwasm_std::{to_json_binary, CosmosMsg, Empty, WasmMsg};
use dao_proposal_single::query::ProposalResponse;
use dao_voting::voting::Vote;
use test_context::test_context;

use crate::helpers::{
    chain::Chain,
    helper::{create_dao, create_proposal, execute, stake_tokens, vote},
};

use super::dao_voting_cw721_staked_test::instantiate_cw721_base;

fn mint_mint_mint_mint(cw721: &str, owner: &str, mints: u64) -> Vec<CosmosMsg> {
    (0..mints)
        .map(|mint| {
            WasmMsg::Execute {
                contract_addr: cw721.to_string(),
                msg: to_json_binary(&cw721_base::msg::ExecuteMsg::Mint::<Empty, Empty>{
                        token_id: mint.to_string(),
                        owner: owner.to_string(),
                        token_uri: Some("https://bafkreibufednctf2f2bpduiibgkvpqcw5rtdmhqh2htqx3qbdnji4h55hy.ipfs.nftstorage.link".to_string()),
                        extension: Empty::default(),
                    },
                )
                .unwrap(),
                funds: vec![],
            }
            .into()
        })
        .collect()
}

/// tests that the maximum number of NFTs creatable in a proposal does
/// not decrease over time. this test failing means that our proposal
/// gas usage as gotten worse.
#[test_context(Chain)]
#[test]
#[ignore]
fn how_many_nfts_can_be_minted_in_one_proposal(chain: &mut Chain) {
    let user_addr = chain.users["user1"].account.address.clone();
    let user_key = chain.users["user1"].key.clone();

    let dao = create_dao(chain, None, "create_dao", user_addr.clone(), &user_key).unwrap();
    let cw721 = instantiate_cw721_base(chain, &user_key, &dao.addr);

    // limit selected with tuning method below.
    let msgs = mint_mint_mint_mint(&cw721, &user_addr, 55);
    stake_tokens(chain, 1, &user_key);
    let ProposalResponse { id, .. } = create_proposal(chain, msgs, &user_key).unwrap();
    vote(chain, id, Vote::Yes, &user_key);
    execute(chain, id, &user_key);

    // for re-tuning the limit, this may be helpful:
    //
    // ```
    // for i in 11..20 {
    //     let msgs = mint_mint_mint_mint(&cw721, &user_addr, 5 * i);
    //     stake_tokens(chain, 1, &user_key);
    //     let ProposalResponse { id, .. } = create_proposal(chain, msgs, &user_key).unwrap();
    //     vote(chain, id, Vote::Yes, &user_key);
    //     execute(chain, id, &user_key);
    //     eprintln!("minted {}", 5 * 11);
    // }
    // ```
}
