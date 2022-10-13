import codegen from '@cosmwasm/ts-codegen';

enum OutputType {
    contracts = "contracts",
    packages = "packages",
    proposal = "proposal",
    staking = "staking",
    voting = "voting",
    "pre-propose" = "pre-propose",
    external = "external",
}

codegen({
    contracts: [
        {
            name: 'cwd-core',
            dir: `../${OutputType.contracts}/cwd-core/schema`
        },
    ],
    outPath: `./${OutputType.contracts}/cwd-core`,
}).then(() => {
    console.log('cwd-core done!');
});
codegen({
    contracts: [
        {
            name: 'cwd-admin-factory',
            dir: `../${OutputType.contracts}/${OutputType.external}/cw-admin-factory/schema`
        },
    ],
    outPath: `./${OutputType.contracts}/cw-admin-factory`,
}).then(() => {
    console.log('cw-admin-factory done!');
});
codegen({
    contracts: [
        {
            name: 'cwd-token-swap',
            dir: `../${OutputType.contracts}/${OutputType.external}/cw-token-swap/schema`
        },
    ],
    outPath: `./${OutputType.contracts}/cw-token-swap`,
}).then(() => {
    console.log('cw-token-swap done!');
});
codegen({
    contracts: [
        {
            name: 'cwd-pre-propose-multiple',
            dir: `../${OutputType.contracts}/${OutputType['pre-propose']}/cwd-pre-propose-multiple/schema`
        },
    ],
    outPath: `./${OutputType.contracts}/cwd-pre-propose-multiple`,
}).then(() => {
    console.log('cwd-pre-propose-multiple done!');
});
codegen({
    contracts: [
        {
            name: 'cwd-pre-propose-single',
            dir: `../${OutputType.contracts}/${OutputType['pre-propose']}/cwd-pre-propose-single/schema`
        },
    ],
    outPath: `./${OutputType.contracts}/cwd-pre-propose-single`,
}).then(() => {
    console.log('cwd-pre-propose-single done!');
});
codegen({
    contracts: [
        {
            name: 'cwd-proposal-multiple',
            dir: `../${OutputType.contracts}/${OutputType.proposal}/cwd-proposal-multiple/schema`
        },
    ],
    outPath: `./${OutputType.contracts}/cwd-proposal-multiple`,
}).then(() => {
    console.log('cwd-proposal-multiple done!');
});
codegen({
    contracts: [
        {
            name: 'cwd-proposal-single',
            dir: `../${OutputType.contracts}/${OutputType.proposal}/cwd-proposal-single/schema`
        },
    ],
    outPath: `./${OutputType.contracts}/cwd-proposal-single`,
}).then(() => {
    console.log('cwd-proposal-single done!');
});
codegen({
    contracts: [
        {
            name: 'cw20-stake',
            dir: `../${OutputType.contracts}/${OutputType.staking}/cw20-stake/schema`
        },
    ],
    outPath: `./${OutputType.contracts}/cw20-stake`,
}).then(() => {
    console.log('cw20-stake done!');
});
codegen({
    contracts: [
        {
            name: 'cw20-stake-external-rewards',
            dir: `../${OutputType.contracts}/${OutputType.staking}/cw20-stake-external-rewards/schema`
        },
    ],
    outPath: `./${OutputType.contracts}/cw20-stake-external-rewards`,
}).then(() => {
    console.log('cw20-stake-external-rewards done!');
});
codegen({
    contracts: [
        {
            name: 'cw20-stake-reward-distributor',
            dir: `../${OutputType.contracts}/${OutputType.staking}/cw20-stake-reward-distributor/schema`
        },
    ],
    outPath: `./${OutputType.contracts}/cw20-stake-reward-distributor`,
}).then(() => {
    console.log('cw20-stake-reward-distributor done!');
});
codegen({
    contracts: [
        {
            name: 'cwd-voting-cw4',
            dir: `../${OutputType.contracts}/${OutputType.voting}/cwd-voting-cw4/schema`
        },
    ],
    outPath: `./${OutputType.contracts}/cwd-voting-cw4`,
}).then(() => {
    console.log('cwd-voting-cw4 done!');
});
codegen({
    contracts: [
        {
            name: 'cwd-voting-cw20-staked',
            dir: `../${OutputType.contracts}/${OutputType.voting}/cwd-voting-cw20-staked/schema`
        },
    ],
    outPath: `./${OutputType.contracts}/cwd-voting-cw20-staked`,
}).then(() => {
    console.log('cwd-voting-cw20-staked done!');
});
codegen({
    contracts: [
        {
            name: 'cwd-voting-cw721-staked',
            dir: `../${OutputType.contracts}/${OutputType.voting}/cwd-voting-cw721-staked/schema`
        },
    ],
    outPath: `./${OutputType.contracts}/cwd-voting-cw721-staked`,
}).then(() => {
    console.log('cwd-voting-cw721-staked done!');
});
codegen({
    contracts: [
        {
            name: 'cwd-voting-native-staked',
            dir: `../${OutputType.contracts}/${OutputType.voting}/cwd-voting-native-staked/schema`
        },
    ],
    outPath: `./${OutputType.contracts}/cwd-voting-native-staked`,
}).then(() => {
    console.log('cwd-voting-native-staked done!');
});
codegen({
    contracts: [
        {
            name: 'cwd-voting-staking-denom-staked',
            dir: `../${OutputType.contracts}/${OutputType.voting}/cwd-voting-staking-denom-staked/schema`
        },
    ],
    outPath: `./${OutputType.contracts}/cwd-voting-staking-denom-staked`,
}).then(() => {
    console.log('cwd-voting-staking-denom-staked done!');
});