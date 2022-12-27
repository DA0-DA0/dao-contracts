use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, BlockInfo, CustomQuery, Deps, StdResult, Storage};
use cw_storage_plus::Map;
use cw_utils::Expiration;

#[cw_serde]
pub struct NftClaimsResponse {
    pub nft_claims: Vec<NftClaim>,
}

#[cw_serde]
pub struct NftClaim {
    pub token_id: String,
    pub release_at: Expiration,
}

impl NftClaim {
    pub fn new(token_id: String, released: Expiration) -> Self {
        NftClaim {
            token_id,
            release_at: released,
        }
    }
}

pub struct NftClaims<'a>(Map<'a, &'a Addr, Vec<NftClaim>>);

impl<'a> NftClaims<'a> {
    pub const fn new(storage_key: &'a str) -> Self {
        NftClaims(Map::new(storage_key))
    }

    /// Creates a number of NFT claims simeltaniously for a given
    /// address.
    ///
    /// # Invariants
    ///
    /// - token_ids must be deduplicated
    /// - token_ids must not contain any IDs which are currently in
    ///   the claims queue for ADDR. This can be ensured by requiring
    ///   that claims are completed before the tokens may be restaked.
    pub fn create_nft_claims(
        &self,
        storage: &mut dyn Storage,
        addr: &Addr,
        token_ids: Vec<String>,
        release_at: Expiration,
    ) -> StdResult<()> {
        self.0.update(storage, addr, |old| -> StdResult<_> {
            Ok(old
                .unwrap_or_default()
                .into_iter()
                .chain(token_ids.into_iter().map(|token_id| NftClaim {
                    token_id,
                    release_at,
                }))
                .collect::<Vec<NftClaim>>())
        })?;
        Ok(())
    }

    /// This iterates over all mature claims for the address, and removes them, up to an optional cap.
    /// it removes the finished claims and returns the total amount of tokens to be released.
    pub fn claim_nfts(
        &self,
        storage: &mut dyn Storage,
        addr: &Addr,
        block: &BlockInfo,
    ) -> StdResult<Vec<String>> {
        let mut to_send = vec![];
        self.0.update(storage, addr, |nft_claims| -> StdResult<_> {
            let (_send, waiting): (Vec<_>, _) =
                nft_claims.unwrap_or_default().into_iter().partition(|c| {
                    // if mature and we can pay fully, then include in _send
                    if c.release_at.is_expired(block) {
                        to_send.push(c.token_id.clone());
                        true
                    } else {
                        // not to send, leave in waiting and save again
                        false
                    }
                });
            Ok(waiting)
        })?;
        Ok(to_send)
    }

    pub fn query_claims<Q: CustomQuery>(
        &self,
        deps: Deps<Q>,
        address: &Addr,
    ) -> StdResult<NftClaimsResponse> {
        let nft_claims = self.0.may_load(deps.storage, address)?.unwrap_or_default();
        Ok(NftClaimsResponse { nft_claims })
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env},
        Order,
    };

    use super::*;
    const TEST_BAYC_TOKEN_ID: &str = "BAYC";
    const TEST_CRYPTO_PUNKS_TOKEN_ID: &str = "CRYPTOPUNKS";
    const TEST_EXPIRATION: Expiration = Expiration::AtHeight(10);

    #[test]
    fn can_create_claim() {
        let claim = NftClaim::new(TEST_BAYC_TOKEN_ID.to_string(), TEST_EXPIRATION);
        assert_eq!(claim.token_id, TEST_BAYC_TOKEN_ID.to_string());
        assert_eq!(claim.release_at, TEST_EXPIRATION);
    }

    #[test]
    fn can_create_claims() {
        let deps = mock_dependencies();
        let claims = NftClaims::new("claims");
        // Assert that claims creates a map and there are no keys in the map.
        assert_eq!(
            claims
                .0
                .range_raw(&deps.storage, None, None, Order::Ascending)
                .collect::<StdResult<Vec<_>>>()
                .unwrap()
                .len(),
            0
        );
    }

    #[test]
    fn check_create_claim_updates_map() {
        let mut deps = mock_dependencies();
        let claims = NftClaims::new("claims");

        claims
            .create_nft_claims(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                vec![TEST_BAYC_TOKEN_ID.into()],
                TEST_EXPIRATION,
            )
            .unwrap();

        // Assert that claims creates a map and there is one claim for the address.
        let saved_claims = claims
            .0
            .load(deps.as_mut().storage, &Addr::unchecked("addr"))
            .unwrap();
        assert_eq!(saved_claims.len(), 1);
        assert_eq!(saved_claims[0].token_id, TEST_BAYC_TOKEN_ID.to_string());
        assert_eq!(saved_claims[0].release_at, TEST_EXPIRATION);

        // Adding another claim to same address, make sure that both claims are saved.
        claims
            .create_nft_claims(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                vec![TEST_CRYPTO_PUNKS_TOKEN_ID.into()],
                TEST_EXPIRATION,
            )
            .unwrap();

        // Assert that both claims exist for the address.
        let saved_claims = claims
            .0
            .load(deps.as_mut().storage, &Addr::unchecked("addr"))
            .unwrap();
        assert_eq!(saved_claims.len(), 2);
        assert_eq!(saved_claims[0].token_id, TEST_BAYC_TOKEN_ID.to_string());
        assert_eq!(saved_claims[0].release_at, TEST_EXPIRATION);
        assert_eq!(
            saved_claims[1].token_id,
            TEST_CRYPTO_PUNKS_TOKEN_ID.to_string()
        );
        assert_eq!(saved_claims[1].release_at, TEST_EXPIRATION);

        // Adding another claim to different address, make sure that other address only has one claim.
        claims
            .create_nft_claims(
                deps.as_mut().storage,
                &Addr::unchecked("addr2"),
                vec![TEST_CRYPTO_PUNKS_TOKEN_ID.to_string()],
                TEST_EXPIRATION,
            )
            .unwrap();

        // Assert that both claims exist for the address.
        let saved_claims = claims
            .0
            .load(deps.as_mut().storage, &Addr::unchecked("addr"))
            .unwrap();

        let saved_claims_addr2 = claims
            .0
            .load(deps.as_mut().storage, &Addr::unchecked("addr2"))
            .unwrap();
        assert_eq!(saved_claims.len(), 2);
        assert_eq!(saved_claims_addr2.len(), 1);
    }

    #[test]
    fn test_claim_tokens_with_no_claims() {
        let mut deps = mock_dependencies();
        let claims = NftClaims::new("claims");

        let nfts = claims
            .claim_nfts(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                &mock_env().block,
            )
            .unwrap();
        let saved_claims = claims
            .0
            .load(deps.as_mut().storage, &Addr::unchecked("addr"))
            .unwrap();

        assert_eq!(nfts.len(), 0);
        assert_eq!(saved_claims.len(), 0);
    }

    #[test]
    fn test_claim_tokens_with_no_released_claims() {
        let mut deps = mock_dependencies();
        let claims = NftClaims::new("claims");

        claims
            .create_nft_claims(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                vec![TEST_CRYPTO_PUNKS_TOKEN_ID.to_string()],
                Expiration::AtHeight(10),
            )
            .unwrap();

        claims
            .create_nft_claims(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                vec![TEST_BAYC_TOKEN_ID.to_string()],
                Expiration::AtHeight(100),
            )
            .unwrap();

        let mut env = mock_env();
        env.block.height = 0;
        // the address has two claims however they are both not expired
        let nfts = claims
            .claim_nfts(deps.as_mut().storage, &Addr::unchecked("addr"), &env.block)
            .unwrap();

        let saved_claims = claims
            .0
            .load(deps.as_mut().storage, &Addr::unchecked("addr"))
            .unwrap();

        assert_eq!(nfts.len(), 0);
        assert_eq!(saved_claims.len(), 2);
        assert_eq!(
            saved_claims[0].token_id,
            TEST_CRYPTO_PUNKS_TOKEN_ID.to_string()
        );
        assert_eq!(saved_claims[0].release_at, Expiration::AtHeight(10));
        assert_eq!(saved_claims[1].token_id, TEST_BAYC_TOKEN_ID.to_string());
        assert_eq!(saved_claims[1].release_at, Expiration::AtHeight(100));
    }

    #[test]
    fn test_claim_tokens_with_one_released_claim() {
        let mut deps = mock_dependencies();
        let claims = NftClaims::new("claims");

        claims
            .create_nft_claims(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                vec![TEST_BAYC_TOKEN_ID.to_string()],
                Expiration::AtHeight(10),
            )
            .unwrap();

        claims
            .create_nft_claims(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                vec![TEST_CRYPTO_PUNKS_TOKEN_ID.to_string()],
                Expiration::AtHeight(100),
            )
            .unwrap();

        let mut env = mock_env();
        env.block.height = 20;
        // the address has two claims and the first one can be released
        let nfts = claims
            .claim_nfts(deps.as_mut().storage, &Addr::unchecked("addr"), &env.block)
            .unwrap();

        let saved_claims = claims
            .0
            .load(deps.as_mut().storage, &Addr::unchecked("addr"))
            .unwrap();

        assert_eq!(nfts.len(), 1);
        assert_eq!(nfts[0], TEST_BAYC_TOKEN_ID.to_string());
        assert_eq!(saved_claims.len(), 1);
        assert_eq!(
            saved_claims[0].token_id,
            TEST_CRYPTO_PUNKS_TOKEN_ID.to_string()
        );
        assert_eq!(saved_claims[0].release_at, Expiration::AtHeight(100));
    }

    #[test]
    fn test_claim_tokens_with_all_released_claims() {
        let mut deps = mock_dependencies();
        let claims = NftClaims::new("claims");

        claims
            .create_nft_claims(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                vec![TEST_BAYC_TOKEN_ID.to_string()],
                Expiration::AtHeight(10),
            )
            .unwrap();

        claims
            .create_nft_claims(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                vec![TEST_CRYPTO_PUNKS_TOKEN_ID.to_string()],
                Expiration::AtHeight(100),
            )
            .unwrap();

        let mut env = mock_env();
        env.block.height = 1000;
        // the address has two claims and both can be released
        let nfts = claims
            .claim_nfts(deps.as_mut().storage, &Addr::unchecked("addr"), &env.block)
            .unwrap();

        let saved_claims = claims
            .0
            .load(deps.as_mut().storage, &Addr::unchecked("addr"))
            .unwrap();

        assert_eq!(
            nfts,
            vec![
                TEST_BAYC_TOKEN_ID.to_string(),
                TEST_CRYPTO_PUNKS_TOKEN_ID.to_string()
            ]
        );
        assert_eq!(saved_claims.len(), 0);
    }

    #[test]
    fn test_query_claims_returns_correct_claims() {
        let mut deps = mock_dependencies();
        let claims = NftClaims::new("claims");

        claims
            .create_nft_claims(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                vec![TEST_CRYPTO_PUNKS_TOKEN_ID.to_string()],
                Expiration::AtHeight(10),
            )
            .unwrap();

        let queried_claims = claims
            .query_claims(deps.as_ref(), &Addr::unchecked("addr"))
            .unwrap();
        let saved_claims = claims
            .0
            .load(deps.as_mut().storage, &Addr::unchecked("addr"))
            .unwrap();
        assert_eq!(queried_claims.nft_claims, saved_claims);
    }

    #[test]
    fn test_query_claims_returns_empty_for_non_existent_user() {
        let mut deps = mock_dependencies();
        let claims = NftClaims::new("claims");

        claims
            .create_nft_claims(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                vec![TEST_CRYPTO_PUNKS_TOKEN_ID.to_string()],
                Expiration::AtHeight(10),
            )
            .unwrap();

        let queried_claims = claims
            .query_claims(deps.as_ref(), &Addr::unchecked("addr2"))
            .unwrap();

        assert_eq!(queried_claims.nft_claims.len(), 0);
    }
}
