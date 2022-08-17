package tokenfactory_issuer_test

import (
	"encoding/json"
	"fmt"

	sdk "github.com/cosmos/cosmos-sdk/types"

	stdMath "github.com/cosmwasm/cosmwasm-go/std/math"

	banktypes "github.com/cosmos/cosmos-sdk/x/bank/types"
)

type mintMsgTest struct {
	mintMsg    ExecuteMsg
	sender     sdk.AccAddress
	expectPass bool
}

func (suite *KeeperTestSuite) TestMint() {
	for _, tc := range []struct {
		desc             string
		minterAllowances map[string]uint64
		mintMsgs         []mintMsgTest
	}{
		{
			desc:             "no minting allowance cannot mint",
			minterAllowances: map[string]uint64{},
			mintMsgs: []mintMsgTest{
				{
					mintMsg: ExecuteMsg{Mint: &Mint{
						ToAddress: suite.TestAccs[0].String(),
						Amount:    stdMath.NewUint128FromUint64(1),
					}},
					sender:     suite.TestAccs[0],
					expectPass: false,
				},
			},
		},
		{
			desc:             "can mint up to allowance but not more",
			minterAllowances: map[string]uint64{suite.TestAccs[0].String(): 1000},
			mintMsgs: []mintMsgTest{
				{
					mintMsg: ExecuteMsg{Mint: &Mint{
						ToAddress: suite.TestAccs[0].String(),
						Amount:    stdMath.NewUint128FromUint64(900),
					}},
					sender:     suite.TestAccs[0],
					expectPass: true,
				},
				{
					mintMsg: ExecuteMsg{Mint: &Mint{
						ToAddress: suite.TestAccs[0].String(),
						Amount:    stdMath.NewUint128FromUint64(900),
					}},
					sender:     suite.TestAccs[0],
					expectPass: false,
				},
			},
		},
	} {
		suite.Run(fmt.Sprintf("Case %s", tc.desc), func() {
			// setup test
			suite.SetupTest()
			denom, contractAddr := suite.CreateTokenAndContract()

			// give minter allowances
			for addr, allowance := range tc.minterAllowances {
				setMinterMsg, err := json.Marshal(ExecuteMsg{SetMinter: &SetMinter{Address: addr, Allowance: stdMath.NewUint128FromUint64(allowance)}})
				suite.Require().NoError(err, "test %v", tc.desc)
				_, err = suite.contractKeeper.Execute(suite.Ctx, contractAddr, suite.TestAccs[0], setMinterMsg, sdk.NewCoins())
				suite.Require().NoError(err, "test %v", tc.desc)
			}

			balances := map[string]stdMath.Uint128{}

			for _, msgTc := range tc.mintMsgs {
				msg, err := json.Marshal(msgTc.mintMsg)
				suite.Require().NoError(err, "test %v", tc.desc)

				_, err = suite.contractKeeper.Execute(suite.Ctx, contractAddr, msgTc.sender, msg, sdk.NewCoins())
				if msgTc.expectPass {
					suite.Require().NoError(err)

					// increment expected balance
					balances[msgTc.mintMsg.Mint.ToAddress] = balances[msgTc.mintMsg.Mint.ToAddress].Add(msgTc.mintMsg.Mint.Amount)

					// make sure queried balance equals expected balance
					toAddr, _ := sdk.AccAddressFromBech32(msgTc.mintMsg.Mint.ToAddress)
					bal := suite.App.BankKeeper.GetBalance(suite.Ctx, toAddr, denom).Amount.Uint64()
					suite.Require().True(balances[msgTc.mintMsg.Mint.ToAddress].Equals64(bal))
				} else {
					suite.Require().Error(err)
				}
			}
		})
	}
}

func (suite *KeeperTestSuite) TestFreeze() {
	for _, tc := range []struct {
		desc       string
		frozen     bool
		bankMsg    func(denom string) *banktypes.MsgSend
		expectPass bool
	}{
		{
			desc:   "if contract is not frozen, should allow transfer of the denom",
			frozen: false,
			bankMsg: func(denom string) *banktypes.MsgSend {
				return banktypes.NewMsgSend(suite.TestAccs[0], suite.TestAccs[1], sdk.NewCoins(sdk.NewInt64Coin(denom, 1)))
			},
			expectPass: true,
		},
		{
			desc:   "if contract is not frozen, should allow transfer of multidenom",
			frozen: false,
			bankMsg: func(denom string) *banktypes.MsgSend {
				return banktypes.NewMsgSend(suite.TestAccs[0], suite.TestAccs[1], sdk.NewCoins(
					sdk.NewInt64Coin(denom, 1),
					sdk.NewInt64Coin("uosmo", 1),
				))
			},
			expectPass: true,
		},
		{
			desc:   "if contract is frozen, should not transfer of the denom",
			frozen: true,
			bankMsg: func(denom string) *banktypes.MsgSend {
				return banktypes.NewMsgSend(suite.TestAccs[0], suite.TestAccs[1], sdk.NewCoins(
					sdk.NewInt64Coin(denom, 1),
				))
			},
			expectPass: false,
		},
		{
			desc:   "if contract is frozen, should allow transfer of a different denom",
			frozen: true,
			bankMsg: func(denom string) *banktypes.MsgSend {
				return banktypes.NewMsgSend(suite.TestAccs[0], suite.TestAccs[1], sdk.NewCoins(
					sdk.NewInt64Coin("uosmo", 1),
				))
			},
			expectPass: true,
		},

		{
			desc:   "if contract is frozen, should not transaction of multidenom",
			frozen: true,
			bankMsg: func(denom string) *banktypes.MsgSend {
				return banktypes.NewMsgSend(suite.TestAccs[0], suite.TestAccs[1], sdk.NewCoins(
					sdk.NewInt64Coin(denom, 1),
					sdk.NewInt64Coin("uosmo", 1),
				))
			},
			expectPass: false,
		},
	} {
		suite.Run(fmt.Sprintf("Case %s", tc.desc), func() {
			// setup test
			suite.SetupTest()
			denom, contractAddr := suite.CreateTokenAndContract()

			// give mint permissions to testAcc0
			setMinterMsg, err := json.Marshal(ExecuteMsg{SetMinter: &SetMinter{Address: suite.TestAccs[0].String(), Allowance: stdMath.MaxUint128()}})
			suite.Require().NoError(err, "test %v", tc.desc)

			_, err = suite.contractKeeper.Execute(suite.Ctx, contractAddr, suite.TestAccs[0], setMinterMsg, sdk.NewCoins())
			suite.Require().NoError(err, "test %v", tc.desc)

			// mint 100000 coins to testAcc0
			mintMsg, err := json.Marshal(ExecuteMsg{Mint: &Mint{ToAddress: suite.TestAccs[0].String(), Amount: stdMath.NewUint128FromUint64(1000000)}})
			suite.Require().NoError(err, "test %v", tc.desc)
			_, err = suite.contractKeeper.Execute(suite.Ctx, contractAddr, suite.TestAccs[0], mintMsg, sdk.NewCoins())
			suite.Require().NoError(err, "test %v", err)

			// give testAcc0 freeze permissions
			setFreezerMsg, err := json.Marshal(ExecuteMsg{SetFreezer: &SetFreezer{Address: suite.TestAccs[0].String(), Status: true}})
			suite.Require().NoError(err, "test %v", tc.desc)
			_, err = suite.contractKeeper.Execute(suite.Ctx, contractAddr, suite.TestAccs[0], setFreezerMsg, sdk.NewCoins())
			suite.Require().NoError(err, "test %v", tc.desc)

			// if should freeze
			if tc.frozen {
				// freeze contract
				freezeMsg, err := json.Marshal(ExecuteMsg{Freeze: &Freeze{Status: true}})
				_, err = suite.contractKeeper.Execute(suite.Ctx, contractAddr, suite.TestAccs[0], freezeMsg, sdk.NewCoins())
				suite.Require().NoError(err, "test %v", tc.desc)
			} else {
				freezeMsg, err := json.Marshal(ExecuteMsg{Freeze: &Freeze{Status: false}})
				_, err = suite.contractKeeper.Execute(suite.Ctx, contractAddr, suite.TestAccs[0], freezeMsg, sdk.NewCoins())
				suite.Require().NoError(err, "test %v", tc.desc)
			}

			// // TODO: use query to make sure asset is frozen

			_, err = suite.bankMsgServer.Send(sdk.WrapSDKContext(suite.Ctx),
				tc.bankMsg(denom),
			)

			if tc.expectPass {
				suite.Require().NoError(err, "test: %v", tc.desc)
			} else {
				suite.Require().Error(err, "test: %v", tc.desc)
			}
		})
	}
}
