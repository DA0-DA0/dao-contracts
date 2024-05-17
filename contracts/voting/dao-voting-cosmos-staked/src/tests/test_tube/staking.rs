use osmosis_std::types::cosmos::staking::v1beta1::{
    MsgCreateValidator, MsgCreateValidatorResponse, MsgDelegate, MsgDelegateResponse,
    MsgUndelegate, MsgUndelegateResponse,
};
use osmosis_test_tube::{fn_execute, Module, Runner};

pub struct Staking<'a, R: Runner<'a>> {
    runner: &'a R,
}

impl<'a, R: Runner<'a>> Module<'a, R> for Staking<'a, R> {
    fn new(runner: &'a R) -> Self {
        Self { runner }
    }
}

impl<'a, R> Staking<'a, R>
where
    R: Runner<'a>,
{
    fn_execute! {
        pub delegate: MsgDelegate["/cosmos.staking.v1beta1.MsgDelegate"] => MsgDelegateResponse
    }

    fn_execute! {
        pub undelegate: MsgUndelegate["/cosmos.staking.v1beta1.MsgUndelegate"] => MsgUndelegateResponse
    }

    fn_execute! {
        pub create_validator: MsgCreateValidator["/cosmos.staking.v1beta1.MsgCreateValidator"] => MsgCreateValidatorResponse
    }
}
