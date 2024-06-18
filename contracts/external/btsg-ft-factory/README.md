# btsg-ft-factory

Serves as a factory that issues new
[fantokens](https://github.com/bitsongofficial/go-bitsong/tree/main/x/fantoken)
on BitSong and returns their denom for use with the
[dao-voting-token-staked](../../voting/dao-voting-token-staked) voting module
contract.

Instantiation and execution are permissionless. All DAOs will use the same
factory and execute `Issue` to create new fantokens.
