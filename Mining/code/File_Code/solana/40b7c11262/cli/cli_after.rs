use crate::{
    checks::*,
    cli_output::{CliAccount, CliSignOnlyData, CliSignature, OutputFormat},
    cluster_query::*,
    display::println_name_value,
    nonce::{self, *},
    offline::{blockhash_query::BlockhashQuery, *},
    spend_utils::*,
    stake::*,
    storage::*,
    validator_info::*,
    vote::*,
};
use chrono::prelude::*;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use log::*;
use num_traits::FromPrimitive;
use serde_json::{self, json, Value};
use solana_budget_program::budget_instruction::{self, BudgetError};
use solana_clap_utils::{
    commitment::{commitment_arg_with_default, COMMITMENT_ARG},
    input_parsers::*,
    input_validators::*,
    keypair::signer_from_path,
    offline::SIGN_ONLY_ARG,
    ArgConstant,
};
use solana_client::{
    client_error::{ClientError, ClientErrorKind, Result as ClientResult},
    rpc_client::RpcClient,
    rpc_config::RpcLargestAccountsFilter,
    rpc_response::{RpcAccount, RpcKeyedAccount},
};
#[cfg(not(test))]
use solana_faucet::faucet::request_airdrop_transaction;
#[cfg(test)]
use solana_faucet::faucet_mock::request_airdrop_transaction;
use solana_remote_wallet::remote_wallet::RemoteWalletManager;
use solana_sdk::{
    bpf_loader,
    clock::{Epoch, Slot},
    commitment_config::CommitmentConfig,
    fee_calculator::FeeCalculator,
    hash::Hash,
    instruction::InstructionError,
    loader_instruction,
    message::Message,
    native_token::lamports_to_sol,
    program_utils::DecodeError,
    pubkey::{Pubkey, MAX_SEED_LEN},
    signature::{Keypair, Signature, Signer, SignerError},
    system_instruction::{self, SystemError},
    system_program,
    transaction::{Transaction, TransactionError},
};
use solana_stake_program::{
    stake_instruction::LockupArgs,
    stake_state::{Lockup, StakeAuthorize},
};
use solana_storage_program::storage_instruction::StorageAccountType;
use solana_transaction_status::{EncodedTransaction, TransactionEncoding};
use solana_vote_program::vote_state::VoteAuthorize;
use std::{
    error,
    fmt::Write as FmtWrite,
    fs::File,
    io::{Read, Write},
    net::{IpAddr, SocketAddr},
    sync::Arc,
    thread::sleep,
    time::Duration,
};
use thiserror::Error;
use url::Url;

pub type CliSigners = Vec<Box<dyn Signer>>;
pub type SignerIndex = usize;
pub(crate) struct CliSignerInfo {
    pub signers: CliSigners,
}

impl CliSignerInfo {
    pub(crate) fn index_of(&self, pubkey: Option<Pubkey>) -> Option<usize> {
        if let Some(pubkey) = pubkey {
            self.signers
                .iter()
                .position(|signer| signer.pubkey() == pubkey)
        } else {
            Some(0)
        }
    }
}

pub(crate) fn generate_unique_signers(
    bulk_signers: Vec<Option<Box<dyn Signer>>>,
    matches: &ArgMatches<'_>,
    default_signer_path: &str,
    wallet_manager: &mut Option<Arc<RemoteWalletManager>>,
) -> Result<CliSignerInfo, Box<dyn error::Error>> {
    let mut unique_signers = vec![];

    // Determine if the default signer is needed
    if bulk_signers.iter().any(|signer| signer.is_none()) {
        let default_signer =
            signer_from_path(matches, default_signer_path, "keypair", wallet_manager)?;
        unique_signers.push(default_signer);
    }

    for signer in bulk_signers.into_iter() {
        if let Some(signer) = signer {
            if !unique_signers.iter().any(|s| s == &signer) {
                unique_signers.push(signer);
            }
        }
    }
    Ok(CliSignerInfo {
        signers: unique_signers,
    })
}

const DATA_CHUNK_SIZE: usize = 229; // Keep program chunks under PACKET_DATA_SIZE

pub const FEE_PAYER_ARG: ArgConstant<'static> = ArgConstant {
    name: "fee_payer",
    long: "fee-payer",
    help: "Specify the fee-payer account. This may be a keypair file, the ASK keyword \n\
           or the pubkey of an offline signer, provided an appropriate --signer argument \n\
           is also passed. Defaults to the client keypair.",
};

pub fn fee_payer_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name(FEE_PAYER_ARG.name)
        .long(FEE_PAYER_ARG.long)
        .takes_value(true)
        .value_name("KEYPAIR")
        .validator(is_valid_signer)
        .help(FEE_PAYER_ARG.help)
}

#[derive(Debug)]
pub struct KeypairEq(Keypair);

impl From<Keypair> for KeypairEq {
    fn from(keypair: Keypair) -> Self {
        Self(keypair)
    }
}

impl PartialEq for KeypairEq {
    fn eq(&self, other: &Self) -> bool {
        self.pubkey() == other.pubkey()
    }
}

impl std::ops::Deref for KeypairEq {
    type Target = Keypair;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub fn nonce_authority_arg<'a, 'b>() -> Arg<'a, 'b> {
    nonce::nonce_authority_arg().requires(NONCE_ARG.name)
}

#[derive(Default, Debug, PartialEq)]
pub struct PayCommand {
    pub amount: SpendAmount,
    pub to: Pubkey,
    pub timestamp: Option<DateTime<Utc>>,
    pub timestamp_pubkey: Option<Pubkey>,
    pub witnesses: Option<Vec<Pubkey>>,
    pub cancelable: bool,
    pub sign_only: bool,
    pub blockhash_query: BlockhashQuery,
    pub nonce_account: Option<Pubkey>,
    pub nonce_authority: SignerIndex,
}

#[derive(Debug, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum CliCommand {
    // Cluster Query Commands
    Catchup {
        node_pubkey: Pubkey,
        node_json_rpc_url: Option<String>,
        commitment_config: CommitmentConfig,
        follow: bool,
    },
    ClusterDate,
    ClusterVersion,
    CreateAddressWithSeed {
        from_pubkey: Option<Pubkey>,
        seed: String,
        program_id: Pubkey,
    },
    Fees,
    GetBlockTime {
        slot: Option<Slot>,
    },
    GetEpochInfo {
        commitment_config: CommitmentConfig,
    },
    GetGenesisHash,
    GetEpoch {
        commitment_config: CommitmentConfig,
    },
    GetSlot {
        commitment_config: CommitmentConfig,
    },
    LargestAccounts {
        commitment_config: CommitmentConfig,
        filter: Option<RpcLargestAccountsFilter>,
    },
    Supply {
        commitment_config: CommitmentConfig,
        print_accounts: bool,
    },
    TotalSupply {
        commitment_config: CommitmentConfig,
    },
    GetTransactionCount {
        commitment_config: CommitmentConfig,
    },
    LeaderSchedule,
    LiveSlots,
    Ping {
        lamports: u64,
        interval: Duration,
        count: Option<u64>,
        timeout: Duration,
        commitment_config: CommitmentConfig,
    },
    ShowBlockProduction {
        epoch: Option<Epoch>,
        slot_limit: Option<u64>,
    },
    ShowGossip,
    ShowStakes {
        use_lamports_unit: bool,
        vote_account_pubkeys: Option<Vec<Pubkey>>,
    },
    ShowValidators {
        use_lamports_unit: bool,
        commitment_config: CommitmentConfig,
    },
    TransactionHistory {
        address: Pubkey,
        end_slot: Option<Slot>, // None == latest slot
        slot_limit: u64,
    },
    // Nonce commands
    AuthorizeNonceAccount {
        nonce_account: Pubkey,
        nonce_authority: SignerIndex,
        new_authority: Pubkey,
    },
    CreateNonceAccount {
        nonce_account: SignerIndex,
        seed: Option<String>,
        nonce_authority: Option<Pubkey>,
        amount: SpendAmount,
    },
    GetNonce(Pubkey),
    NewNonce {
        nonce_account: Pubkey,
        nonce_authority: SignerIndex,
    },
    ShowNonceAccount {
        nonce_account_pubkey: Pubkey,
        use_lamports_unit: bool,
    },
    WithdrawFromNonceAccount {
        nonce_account: Pubkey,
        nonce_authority: SignerIndex,
        destination_account_pubkey: Pubkey,
        lamports: u64,
    },
    // Program Deployment
    Deploy(String),
    // Stake Commands
    CreateStakeAccount {
        stake_account: SignerIndex,
        seed: Option<String>,
        staker: Option<Pubkey>,
        withdrawer: Option<Pubkey>,
        lockup: Lockup,
        amount: SpendAmount,
        sign_only: bool,
        blockhash_query: BlockhashQuery,
        nonce_account: Option<Pubkey>,
        nonce_authority: SignerIndex,
        fee_payer: SignerIndex,
        from: SignerIndex,
    },
    DeactivateStake {
        stake_account_pubkey: Pubkey,
        stake_authority: SignerIndex,
        sign_only: bool,
        blockhash_query: BlockhashQuery,
        nonce_account: Option<Pubkey>,
        nonce_authority: SignerIndex,
        fee_payer: SignerIndex,
    },
    DelegateStake {
        stake_account_pubkey: Pubkey,
        vote_account_pubkey: Pubkey,
        stake_authority: SignerIndex,
        force: bool,
        sign_only: bool,
        blockhash_query: BlockhashQuery,
        nonce_account: Option<Pubkey>,
        nonce_authority: SignerIndex,
        fee_payer: SignerIndex,
    },
    SplitStake {
        stake_account_pubkey: Pubkey,
        stake_authority: SignerIndex,
        sign_only: bool,
        blockhash_query: BlockhashQuery,
        nonce_account: Option<Pubkey>,
        nonce_authority: SignerIndex,
        split_stake_account: SignerIndex,
        seed: Option<String>,
        lamports: u64,
        fee_payer: SignerIndex,
    },
    ShowStakeHistory {
        use_lamports_unit: bool,
    },
    ShowStakeAccount {
        pubkey: Pubkey,
        use_lamports_unit: bool,
    },
    StakeAuthorize {
        stake_account_pubkey: Pubkey,
        new_authorizations: Vec<(StakeAuthorize, Pubkey, SignerIndex)>,
        sign_only: bool,
        blockhash_query: BlockhashQuery,
        nonce_account: Option<Pubkey>,
        nonce_authority: SignerIndex,
        fee_payer: SignerIndex,
    },
    StakeSetLockup {
        stake_account_pubkey: Pubkey,
        lockup: LockupArgs,
        custodian: SignerIndex,
        sign_only: bool,
        blockhash_query: BlockhashQuery,
        nonce_account: Option<Pubkey>,
        nonce_authority: SignerIndex,
        fee_payer: SignerIndex,
    },
    WithdrawStake {
        stake_account_pubkey: Pubkey,
        destination_account_pubkey: Pubkey,
        lamports: u64,
        withdraw_authority: SignerIndex,
        custodian: Option<SignerIndex>,
        sign_only: bool,
        blockhash_query: BlockhashQuery,
        nonce_account: Option<Pubkey>,
        nonce_authority: SignerIndex,
        fee_payer: SignerIndex,
    },
    // Storage Commands
    CreateStorageAccount {
        account_owner: Pubkey,
        storage_account: SignerIndex,
        account_type: StorageAccountType,
    },
    ClaimStorageReward {
        node_account_pubkey: Pubkey,
        storage_account_pubkey: Pubkey,
    },
    ShowStorageAccount(Pubkey),
    // Validator Info Commands
    GetValidatorInfo(Option<Pubkey>),
    SetValidatorInfo {
        validator_info: Value,
        force_keybase: bool,
        info_pubkey: Option<Pubkey>,
    },
    // Vote Commands
    CreateVoteAccount {
        seed: Option<String>,
        identity_account: SignerIndex,
        authorized_voter: Option<Pubkey>,
        authorized_withdrawer: Option<Pubkey>,
        commission: u8,
    },
    ShowVoteAccount {
        pubkey: Pubkey,
        use_lamports_unit: bool,
        commitment_config: CommitmentConfig,
    },
    WithdrawFromVoteAccount {
        vote_account_pubkey: Pubkey,
        destination_account_pubkey: Pubkey,
        withdraw_authority: SignerIndex,
        lamports: u64,
    },
    VoteAuthorize {
        vote_account_pubkey: Pubkey,
        new_authorized_pubkey: Pubkey,
        vote_authorize: VoteAuthorize,
    },
    VoteUpdateValidator {
        vote_account_pubkey: Pubkey,
        new_identity_account: SignerIndex,
    },
    // Wallet Commands
    Address,
    Airdrop {
        faucet_host: Option<IpAddr>,
        faucet_port: u16,
        pubkey: Option<Pubkey>,
        lamports: u64,
    },
    Balance {
        pubkey: Option<Pubkey>,
        use_lamports_unit: bool,
        commitment_config: CommitmentConfig,
    },
    Cancel(Pubkey),
    Confirm(Signature),
    DecodeTransaction(Transaction),
    Pay(PayCommand),
    ResolveSigner(Option<String>),
    ShowAccount {
        pubkey: Pubkey,
        output_file: Option<String>,
        use_lamports_unit: bool,
    },
    TimeElapsed(Pubkey, Pubkey, DateTime<Utc>), // TimeElapsed(to, process_id, timestamp)
    Transfer {
        amount: SpendAmount,
        to: Pubkey,
        from: SignerIndex,
        sign_only: bool,
        no_wait: bool,
        blockhash_query: BlockhashQuery,
        nonce_account: Option<Pubkey>,
        nonce_authority: SignerIndex,
        fee_payer: SignerIndex,
    },
    Witness(Pubkey, Pubkey), // Witness(to, process_id)
}

#[derive(Debug, PartialEq)]
pub struct CliCommandInfo {
    pub command: CliCommand,
    pub signers: CliSigners,
}

#[derive(Debug, Error)]
pub enum CliError {
    #[error("bad parameter: {0}")]
    BadParameter(String),
    #[error(transparent)]
    ClientError(#[from] ClientError),
    #[error("command not recognized: {0}")]
    CommandNotRecognized(String),
    #[error("insufficient funds for fee ({0} SOL)")]
    InsufficientFundsForFee(f64),
    #[error("insufficient funds for spend ({0} SOL)")]
    InsufficientFundsForSpend(f64),
    #[error("insufficient funds for spend ({0} SOL) and fee ({1} SOL)")]
    InsufficientFundsForSpendAndFee(f64, f64),
    #[error(transparent)]
    InvalidNonce(CliNonceError),
    #[error("dynamic program error: {0}")]
    DynamicProgramError(String),
    #[error("rpc request error: {0}")]
    RpcRequestError(String),
    #[error("keypair file not found: {0}")]
    KeypairFileNotFound(String),
}

impl From<Box<dyn error::Error>> for CliError {
    fn from(error: Box<dyn error::Error>) -> Self {
        CliError::DynamicProgramError(error.to_string())
    }
}

impl From<CliNonceError> for CliError {
    fn from(error: CliNonceError) -> Self {
        match error {
            CliNonceError::Client(client_error) => Self::RpcRequestError(client_error),
            _ => Self::InvalidNonce(error),
        }
    }
}

pub enum SettingType {
    Explicit,
    Computed,
    SystemDefault,
}

pub struct CliConfig<'a> {
    pub command: CliCommand,
    pub json_rpc_url: String,
    pub websocket_url: String,
    pub signers: Vec<&'a dyn Signer>,
    pub keypair_path: String,
    pub rpc_client: Option<RpcClient>,
    pub verbose: bool,
    pub output_format: OutputFormat,
}

impl CliConfig<'_> {
    fn default_keypair_path() -> String {
        solana_cli_config::Config::default().keypair_path
    }

    fn default_json_rpc_url() -> String {
        solana_cli_config::Config::default().json_rpc_url
    }

    fn default_websocket_url() -> String {
        solana_cli_config::Config::default().websocket_url
    }

    fn first_nonempty_setting(
        settings: std::vec::Vec<(SettingType, String)>,
    ) -> (SettingType, String) {
        settings
            .into_iter()
            .find(|(_, value)| value != "")
            .expect("no nonempty setting")
    }

    pub fn compute_websocket_url_setting(
        websocket_cmd_url: &str,
        websocket_cfg_url: &str,
        json_rpc_cmd_url: &str,
        json_rpc_cfg_url: &str,
    ) -> (SettingType, String) {
        Self::first_nonempty_setting(vec![
            (SettingType::Explicit, websocket_cmd_url.to_string()),
            (SettingType::Explicit, websocket_cfg_url.to_string()),
            (
                SettingType::Computed,
                solana_cli_config::Config::compute_websocket_url(json_rpc_cmd_url),
            ),
            (
                SettingType::Computed,
                solana_cli_config::Config::compute_websocket_url(json_rpc_cfg_url),
            ),
            (SettingType::SystemDefault, Self::default_websocket_url()),
        ])
    }

    pub fn compute_json_rpc_url_setting(
        json_rpc_cmd_url: &str,
        json_rpc_cfg_url: &str,
    ) -> (SettingType, String) {
        Self::first_nonempty_setting(vec![
            (SettingType::Explicit, json_rpc_cmd_url.to_string()),
            (SettingType::Explicit, json_rpc_cfg_url.to_string()),
            (SettingType::SystemDefault, Self::default_json_rpc_url()),
        ])
    }

    pub fn compute_keypair_path_setting(
        keypair_cmd_path: &str,
        keypair_cfg_path: &str,
    ) -> (SettingType, String) {
        Self::first_nonempty_setting(vec![
            (SettingType::Explicit, keypair_cmd_path.to_string()),
            (SettingType::Explicit, keypair_cfg_path.to_string()),
            (SettingType::SystemDefault, Self::default_keypair_path()),
        ])
    }

    pub(crate) fn pubkey(&self) -> Result<Pubkey, SignerError> {
        if !self.signers.is_empty() {
            self.signers[0].try_pubkey()
        } else {
            Err(SignerError::Custom(
                "Default keypair must be set if pubkey arg not provided".to_string(),
            ))
        }
    }
}

impl Default for CliConfig<'_> {
    fn default() -> CliConfig<'static> {
        CliConfig {
            command: CliCommand::Balance {
                pubkey: Some(Pubkey::default()),
                use_lamports_unit: false,
                commitment_config: CommitmentConfig::default(),
            },
            json_rpc_url: Self::default_json_rpc_url(),
            websocket_url: Self::default_websocket_url(),
            signers: Vec::new(),
            keypair_path: Self::default_keypair_path(),
            rpc_client: None,
            verbose: false,
            output_format: OutputFormat::Display,
        }
    }
}

pub fn parse_command(
    matches: &ArgMatches<'_>,
    default_signer_path: &str,
    wallet_manager: &mut Option<Arc<RemoteWalletManager>>,
) -> Result<CliCommandInfo, Box<dyn error::Error>> {
    let response = match matches.subcommand() {
        // Cluster Query Commands
        ("catchup", Some(matches)) => parse_catchup(matches, wallet_manager),
        ("cluster-date", Some(_matches)) => Ok(CliCommandInfo {
            command: CliCommand::ClusterDate,
            signers: vec![],
        }),
        ("cluster-version", Some(_matches)) => Ok(CliCommandInfo {
            command: CliCommand::ClusterVersion,
            signers: vec![],
        }),
        ("create-address-with-seed", Some(matches)) => {
            parse_create_address_with_seed(matches, default_signer_path, wallet_manager)
        }
        ("fees", Some(_matches)) => Ok(CliCommandInfo {
            command: CliCommand::Fees,
            signers: vec![],
        }),
        ("block-time", Some(matches)) => parse_get_block_time(matches),
        ("epoch-info", Some(matches)) => parse_get_epoch_info(matches),
        ("genesis-hash", Some(_matches)) => Ok(CliCommandInfo {
            command: CliCommand::GetGenesisHash,
            signers: vec![],
        }),
        ("epoch", Some(matches)) => parse_get_epoch(matches),
        ("slot", Some(matches)) => parse_get_slot(matches),
        ("largest-accounts", Some(matches)) => parse_largest_accounts(matches),
        ("supply", Some(matches)) => parse_supply(matches),
        ("total-supply", Some(matches)) => parse_total_supply(matches),
        ("transaction-count", Some(matches)) => parse_get_transaction_count(matches),
        ("leader-schedule", Some(_matches)) => Ok(CliCommandInfo {
            command: CliCommand::LeaderSchedule,
            signers: vec![],
        }),
        ("ping", Some(matches)) => parse_cluster_ping(matches, default_signer_path, wallet_manager),
        ("live-slots", Some(_matches)) => Ok(CliCommandInfo {
            command: CliCommand::LiveSlots,
            signers: vec![],
        }),
        ("block-production", Some(matches)) => parse_show_block_production(matches),
        ("gossip", Some(_matches)) => Ok(CliCommandInfo {
            command: CliCommand::ShowGossip,
            signers: vec![],
        }),
        ("stakes", Some(matches)) => parse_show_stakes(matches, wallet_manager),
        ("validators", Some(matches)) => parse_show_validators(matches),
        ("transaction-history", Some(matches)) => {
            parse_transaction_history(matches, wallet_manager)
        }
        // Nonce Commands
        ("authorize-nonce-account", Some(matches)) => {
            parse_authorize_nonce_account(matches, default_signer_path, wallet_manager)
        }
        ("create-nonce-account", Some(matches)) => {
            parse_nonce_create_account(matches, default_signer_path, wallet_manager)
        }
        ("nonce", Some(matches)) => parse_get_nonce(matches, wallet_manager),
        ("new-nonce", Some(matches)) => {
            parse_new_nonce(matches, default_signer_path, wallet_manager)
        }
        ("nonce-account", Some(matches)) => parse_show_nonce_account(matches, wallet_manager),
        ("withdraw-from-nonce-account", Some(matches)) => {
            parse_withdraw_from_nonce_account(matches, default_signer_path, wallet_manager)
        }
        // Program Deployment
        ("deploy", Some(matches)) => Ok(CliCommandInfo {
            command: CliCommand::Deploy(matches.value_of("program_location").unwrap().to_string()),
            signers: vec![signer_from_path(
                matches,
                default_signer_path,
                "keypair",
                wallet_manager,
            )?],
        }),
        // Stake Commands
        ("create-stake-account", Some(matches)) => {
            parse_stake_create_account(matches, default_signer_path, wallet_manager)
        }
        ("delegate-stake", Some(matches)) => {
            parse_stake_delegate_stake(matches, default_signer_path, wallet_manager)
        }
        ("withdraw-stake", Some(matches)) => {
            parse_stake_withdraw_stake(matches, default_signer_path, wallet_manager)
        }
        ("deactivate-stake", Some(matches)) => {
            parse_stake_deactivate_stake(matches, default_signer_path, wallet_manager)
        }
        ("split-stake", Some(matches)) => {
            parse_split_stake(matches, default_signer_path, wallet_manager)
        }
        ("stake-authorize", Some(matches)) => {
            parse_stake_authorize(matches, default_signer_path, wallet_manager)
        }
        ("stake-set-lockup", Some(matches)) => {
            parse_stake_set_lockup(matches, default_signer_path, wallet_manager)
        }
        ("stake-account", Some(matches)) => parse_show_stake_account(matches, wallet_manager),
        ("stake-history", Some(matches)) => parse_show_stake_history(matches),
        // Storage Commands
        ("create-archiver-storage-account", Some(matches)) => {
            parse_storage_create_archiver_account(matches, default_signer_path, wallet_manager)
        }
        ("create-validator-storage-account", Some(matches)) => {
            parse_storage_create_validator_account(matches, default_signer_path, wallet_manager)
        }
        ("claim-storage-reward", Some(matches)) => {
            parse_storage_claim_reward(matches, default_signer_path, wallet_manager)
        }
        ("storage-account", Some(matches)) => {
            parse_storage_get_account_command(matches, wallet_manager)
        }
        // Validator Info Commands
        ("validator-info", Some(matches)) => match matches.subcommand() {
            ("publish", Some(matches)) => {
                parse_validator_info_command(matches, default_signer_path, wallet_manager)
            }
            ("get", Some(matches)) => parse_get_validator_info_command(matches),
            _ => unreachable!(),
        },
        // Vote Commands
        ("create-vote-account", Some(matches)) => {
            parse_create_vote_account(matches, default_signer_path, wallet_manager)
        }
        ("vote-update-validator", Some(matches)) => {
            parse_vote_update_validator(matches, default_signer_path, wallet_manager)
        }
        ("vote-authorize-voter", Some(matches)) => parse_vote_authorize(
            matches,
            default_signer_path,
            wallet_manager,
            VoteAuthorize::Voter,
        ),
        ("vote-authorize-withdrawer", Some(matches)) => parse_vote_authorize(
            matches,
            default_signer_path,
            wallet_manager,
            VoteAuthorize::Withdrawer,
        ),
        ("vote-account", Some(matches)) => parse_vote_get_account_command(matches, wallet_manager),
        ("withdraw-from-vote-account", Some(matches)) => {
            parse_withdraw_from_vote_account(matches, default_signer_path, wallet_manager)
        }
        // Wallet Commands
        ("address", Some(matches)) => Ok(CliCommandInfo {
            command: CliCommand::Address,
            signers: vec![signer_from_path(
                matches,
                default_signer_path,
                "keypair",
                wallet_manager,
            )?],
        }),
        ("airdrop", Some(matches)) => {
            let faucet_port = matches
                .value_of("faucet_port")
                .unwrap()
                .parse()
                .or_else(|err| {
                    Err(CliError::BadParameter(format!(
                        "Invalid faucet port: {}",
                        err
                    )))
                })?;

            let faucet_host = if let Some(faucet_host) = matches.value_of("faucet_host") {
                Some(solana_net_utils::parse_host(faucet_host).or_else(|err| {
                    Err(CliError::BadParameter(format!(
                        "Invalid faucet host: {}",
                        err
                    )))
                })?)
            } else {
                None
            };
            let pubkey = pubkey_of_signer(matches, "to", wallet_manager)?;
            let signers = if pubkey.is_some() {
                vec![]
            } else {
                vec![signer_from_path(
                    matches,
                    default_signer_path,
                    "keypair",
                    wallet_manager,
                )?]
            };
            let lamports = lamports_of_sol(matches, "amount").unwrap();
            Ok(CliCommandInfo {
                command: CliCommand::Airdrop {
                    faucet_host,
                    faucet_port,
                    pubkey,
                    lamports,
                },
                signers,
            })
        }
        ("balance", Some(matches)) => {
            let pubkey = pubkey_of_signer(matches, "pubkey", wallet_manager)?;
            let commitment_config = commitment_of(matches, COMMITMENT_ARG.long).unwrap();
            let signers = if pubkey.is_some() {
                vec![]
            } else {
                vec![signer_from_path(
                    matches,
                    default_signer_path,
                    "keypair",
                    wallet_manager,
                )?]
            };
            Ok(CliCommandInfo {
                command: CliCommand::Balance {
                    pubkey,
                    use_lamports_unit: matches.is_present("lamports"),
                    commitment_config,
                },
                signers,
            })
        }
        ("cancel", Some(matches)) => {
            let process_id = value_of(matches, "process_id").unwrap();
            let default_signer =
                signer_from_path(matches, default_signer_path, "keypair", wallet_manager)?;

            Ok(CliCommandInfo {
                command: CliCommand::Cancel(process_id),
                signers: vec![default_signer],
            })
        }
        ("confirm", Some(matches)) => match matches.value_of("signature").unwrap().parse() {
            Ok(signature) => Ok(CliCommandInfo {
                command: CliCommand::Confirm(signature),
                signers: vec![],
            }),
            _ => Err(CliError::BadParameter("Invalid signature".to_string())),
        },
        ("decode-transaction", Some(matches)) => {
            let encoded_transaction = EncodedTransaction::Binary(
                matches.value_of("base58_transaction").unwrap().to_string(),
            );
            if let Some(transaction) = encoded_transaction.decode() {
                Ok(CliCommandInfo {
                    command: CliCommand::DecodeTransaction(transaction),
                    signers: vec![],
                })
            } else {
                Err(CliError::BadParameter(
                    "Unable to decode transaction".to_string(),
                ))
            }
        }
        ("pay", Some(matches)) => {
            let amount = SpendAmount::new_from_matches(matches, "amount");
            let to = pubkey_of_signer(matches, "to", wallet_manager)?.unwrap();
            let timestamp = if matches.is_present("timestamp") {
                // Parse input for serde_json
                let date_string = if !matches.value_of("timestamp").unwrap().contains('Z') {
                    format!("\"{}Z\"", matches.value_of("timestamp").unwrap())
                } else {
                    format!("\"{}\"", matches.value_of("timestamp").unwrap())
                };
                Some(serde_json::from_str(&date_string)?)
            } else {
                None
            };
            let timestamp_pubkey = value_of(matches, "timestamp_pubkey");
            let witnesses = values_of(matches, "witness");
            let cancelable = matches.is_present("cancelable");
            let sign_only = matches.is_present(SIGN_ONLY_ARG.name);
            let blockhash_query = BlockhashQuery::new_from_matches(matches);
            let nonce_account = pubkey_of_signer(matches, NONCE_ARG.name, wallet_manager)?;
            let (nonce_authority, nonce_authority_pubkey) =
                signer_of(matches, NONCE_AUTHORITY_ARG.name, wallet_manager)?;

            let payer_provided = None;
            let mut bulk_signers = vec![payer_provided];
            if nonce_account.is_some() {
                bulk_signers.push(nonce_authority);
            }
            let signer_info = generate_unique_signers(
                bulk_signers,
                matches,
                default_signer_path,
                wallet_manager,
            )?;

            Ok(CliCommandInfo {
                command: CliCommand::Pay(PayCommand {
                    amount,
                    to,
                    timestamp,
                    timestamp_pubkey,
                    witnesses,
                    cancelable,
                    sign_only,
                    blockhash_query,
                    nonce_account,
                    nonce_authority: signer_info.index_of(nonce_authority_pubkey).unwrap(),
                }),
                signers: signer_info.signers,
            })
        }
        ("account", Some(matches)) => {
            let account_pubkey =
                pubkey_of_signer(matches, "account_pubkey", wallet_manager)?.unwrap();
            let output_file = matches.value_of("output_file");
            let use_lamports_unit = matches.is_present("lamports");
            Ok(CliCommandInfo {
                command: CliCommand::ShowAccount {
                    pubkey: account_pubkey,
                    output_file: output_file.map(ToString::to_string),
                    use_lamports_unit,
                },
                signers: vec![],
            })
        }
        ("resolve-signer", Some(matches)) => {
            let signer_path = resolve_signer(matches, "signer", wallet_manager)?;
            Ok(CliCommandInfo {
                command: CliCommand::ResolveSigner(signer_path),
                signers: vec![],
            })
        }
        ("send-signature", Some(matches)) => {
            let to = value_of(matches, "to").unwrap();
            let process_id = value_of(matches, "process_id").unwrap();

            let default_signer =
                signer_from_path(matches, default_signer_path, "keypair", wallet_manager)?;

            Ok(CliCommandInfo {
                command: CliCommand::Witness(to, process_id),
                signers: vec![default_signer],
            })
        }
        ("send-timestamp", Some(matches)) => {
            let to = value_of(matches, "to").unwrap();
            let process_id = value_of(matches, "process_id").unwrap();
            let dt = if matches.is_present("datetime") {
                // Parse input for serde_json
                let date_string = if !matches.value_of("datetime").unwrap().contains('Z') {
                    format!("\"{}Z\"", matches.value_of("datetime").unwrap())
                } else {
                    format!("\"{}\"", matches.value_of("datetime").unwrap())
                };
                serde_json::from_str(&date_string)?
            } else {
                Utc::now()
            };
            let default_signer =
                signer_from_path(matches, default_signer_path, "keypair", wallet_manager)?;

            Ok(CliCommandInfo {
                command: CliCommand::TimeElapsed(to, process_id, dt),
                signers: vec![default_signer],
            })
        }
        ("transfer", Some(matches)) => {
            let amount = SpendAmount::new_from_matches(matches, "amount");
            let to = pubkey_of_signer(matches, "to", wallet_manager)?.unwrap();
            let sign_only = matches.is_present(SIGN_ONLY_ARG.name);
            let no_wait = matches.is_present("no_wait");
            let blockhash_query = BlockhashQuery::new_from_matches(matches);
            let nonce_account = pubkey_of_signer(matches, NONCE_ARG.name, wallet_manager)?;
            let (nonce_authority, nonce_authority_pubkey) =
                signer_of(matches, NONCE_AUTHORITY_ARG.name, wallet_manager)?;
            let (fee_payer, fee_payer_pubkey) =
                signer_of(matches, FEE_PAYER_ARG.name, wallet_manager)?;
            let (from, from_pubkey) = signer_of(matches, "from", wallet_manager)?;

            let mut bulk_signers = vec![fee_payer, from];
            if nonce_account.is_some() {
                bulk_signers.push(nonce_authority);
            }

            let signer_info = generate_unique_signers(
                bulk_signers,
                matches,
                default_signer_path,
                wallet_manager,
            )?;

            Ok(CliCommandInfo {
                command: CliCommand::Transfer {
                    amount,
                    to,
                    sign_only,
                    no_wait,
                    blockhash_query,
                    nonce_account,
                    nonce_authority: signer_info.index_of(nonce_authority_pubkey).unwrap(),
                    fee_payer: signer_info.index_of(fee_payer_pubkey).unwrap(),
                    from: signer_info.index_of(from_pubkey).unwrap(),
                },
                signers: signer_info.signers,
            })
        }
        //
        ("", None) => {
            eprintln!("{}", matches.usage());
            Err(CliError::CommandNotRecognized(
                "no subcommand given".to_string(),
            ))
        }
        _ => unreachable!(),
    }?;
    Ok(response)
}

pub type ProcessResult = Result<String, Box<dyn std::error::Error>>;

pub fn get_blockhash_and_fee_calculator(
    rpc_client: &RpcClient,
    sign_only: bool,
    blockhash: Option<Hash>,
) -> Result<(Hash, FeeCalculator), Box<dyn std::error::Error>> {
    Ok(if let Some(blockhash) = blockhash {
        if sign_only {
            (blockhash, FeeCalculator::default())
        } else {
            (blockhash, rpc_client.get_recent_blockhash()?.1)
        }
    } else {
        rpc_client.get_recent_blockhash()?
    })
}

pub fn return_signers(tx: &Transaction, config: &CliConfig) -> ProcessResult {
    let verify_results = tx.verify_with_results();
    let mut signers = Vec::new();
    let mut absent = Vec::new();
    let mut bad_sig = Vec::new();
    tx.signatures
        .iter()
        .zip(tx.message.account_keys.iter())
        .zip(verify_results.into_iter())
        .for_each(|((sig, key), res)| {
            if res {
                signers.push(format!("{}={}", key, sig))
            } else if *sig == Signature::default() {
                absent.push(key.to_string());
            } else {
                bad_sig.push(key.to_string());
            }
        });

    let cli_command = CliSignOnlyData {
        blockhash: tx.message.recent_blockhash.to_string(),
        signers,
        absent,
        bad_sig,
    };

    Ok(config.output_format.formatted_string(&cli_command))
}

pub fn parse_create_address_with_seed(
    matches: &ArgMatches<'_>,
    default_signer_path: &str,
    wallet_manager: &mut Option<Arc<RemoteWalletManager>>,
) -> Result<CliCommandInfo, CliError> {
    let from_pubkey = pubkey_of_signer(matches, "from", wallet_manager)?;
    let signers = if from_pubkey.is_some() {
        vec![]
    } else {
        vec![signer_from_path(
            matches,
            default_signer_path,
            "keypair",
            wallet_manager,
        )?]
    };

    let program_id = match matches.value_of("program_id").unwrap() {
        "NONCE" => system_program::id(),
        "STAKE" => solana_stake_program::id(),
        "STORAGE" => solana_storage_program::id(),
        "VOTE" => solana_vote_program::id(),
        _ => pubkey_of(matches, "program_id").unwrap(),
    };

    let seed = matches.value_of("seed").unwrap().to_string();

    if seed.len() > MAX_SEED_LEN {
        return Err(CliError::BadParameter(
            "Address seed must not be longer than 32 bytes".to_string(),
        ));
    }

    Ok(CliCommandInfo {
        command: CliCommand::CreateAddressWithSeed {
            from_pubkey,
            seed,
            program_id,
        },
        signers,
    })
}

fn process_create_address_with_seed(
    config: &CliConfig,
    from_pubkey: Option<&Pubkey>,
    seed: &str,
    program_id: &Pubkey,
) -> ProcessResult {
    let from_pubkey = if let Some(pubkey) = from_pubkey {
        *pubkey
    } else {
        config.pubkey()?
    };
    let address = Pubkey::create_with_seed(&from_pubkey, seed, program_id)?;
    Ok(address.to_string())
}

fn process_airdrop(
    rpc_client: &RpcClient,
    config: &CliConfig,
    faucet_addr: &SocketAddr,
    pubkey: &Option<Pubkey>,
    lamports: u64,
) -> ProcessResult {
    let pubkey = if let Some(pubkey) = pubkey {
        *pubkey
    } else {
        config.pubkey()?
    };
    println!(
        "Requesting airdrop of {} from {}",
        build_balance_message(lamports, false, true),
        faucet_addr
    );

    request_and_confirm_airdrop(&rpc_client, faucet_addr, &pubkey, lamports, &config)?;

    let current_balance = rpc_client.get_balance(&pubkey)?;

    Ok(build_balance_message(current_balance, false, true))
}

fn process_balance(
    rpc_client: &RpcClient,
    config: &CliConfig,
    pubkey: &Option<Pubkey>,
    use_lamports_unit: bool,
    commitment_config: CommitmentConfig,
) -> ProcessResult {
    let pubkey = if let Some(pubkey) = pubkey {
        *pubkey
    } else {
        config.pubkey()?
    };
    let balance = rpc_client
        .get_balance_with_commitment(&pubkey, commitment_config)?
        .value;
    Ok(build_balance_message(balance, use_lamports_unit, true))
}

fn process_confirm(
    rpc_client: &RpcClient,
    config: &CliConfig,
    signature: &Signature,
) -> ProcessResult {
    match rpc_client.get_signature_status_with_commitment_and_history(
        &signature,
        CommitmentConfig::max(),
        true,
    ) {
        Ok(status) => {
            if let Some(transaction_status) = status {
                if config.verbose {
                    match rpc_client
                        .get_confirmed_transaction(signature, TransactionEncoding::Binary)
                    {
                        Ok(confirmed_transaction) => {
                            println!(
                                "\nTransaction executed in slot {}:",
                                confirmed_transaction.slot
                            );
                            crate::display::println_transaction(
                                &confirmed_transaction
                                    .transaction
                                    .transaction
                                    .decode()
                                    .expect("Successful decode"),
                                &confirmed_transaction.transaction.meta,
                                "  ",
                            );
                        }
                        Err(err) => {
                            println!("Unable to get confirmed transaction details: {}", err)
                        }
                    }
                    println!();
                }

                match transaction_status {
                    Ok(_) => Ok("Confirmed".to_string()),
                    Err(err) => Ok(format!("Transaction failed: {}", err)),
                }
            } else {
                Ok("Not found".to_string())
            }
        }
        Err(err) => Err(CliError::RpcRequestError(format!("Unable to confirm: {}", err)).into()),
    }
}

fn process_decode_transaction(transaction: &Transaction) -> ProcessResult {
    crate::display::println_transaction(transaction, &None, "");
    Ok("".to_string())
}

fn process_show_account(
    rpc_client: &RpcClient,
    config: &CliConfig,
    account_pubkey: &Pubkey,
    output_file: &Option<String>,
    use_lamports_unit: bool,
) -> ProcessResult {
    let account = rpc_client.get_account(account_pubkey)?;
    let data = account.data.clone();
    let cli_account = CliAccount {
        keyed_account: RpcKeyedAccount {
            pubkey: account_pubkey.to_string(),
            account: RpcAccount::encode(account),
        },
        use_lamports_unit,
    };

    let mut account_string = config.output_format.formatted_string(&cli_account);

    if config.output_format == OutputFormat::Display {
        if let Some(output_file) = output_file {
            let mut f = File::create(output_file)?;
            f.write_all(&data)?;
            writeln!(&mut account_string)?;
            writeln!(&mut account_string, "Wrote account data to {}", output_file)?;
        } else if !data.is_empty() {
            use pretty_hex::*;
            writeln!(&mut account_string, "{:?}", data.hex_dump())?;
        }
    }

    Ok(account_string)
}

fn process_deploy(
    rpc_client: &RpcClient,
    config: &CliConfig,
    program_location: &str,
) -> ProcessResult {
    let program_id = Keypair::new();
    let mut file = File::open(program_location).map_err(|err| {
        CliError::DynamicProgramError(format!("Unable to open program file: {}", err))
    })?;
    let mut program_data = Vec::new();
    file.read_to_end(&mut program_data).map_err(|err| {
        CliError::DynamicProgramError(format!("Unable to read program file: {}", err))
    })?;

    // Build transactions to calculate fees
    let mut messages: Vec<&Message> = Vec::new();
    let (blockhash, fee_calculator) = rpc_client.get_recent_blockhash()?;
    let minimum_balance = rpc_client.get_minimum_balance_for_rent_exemption(program_data.len())?;
    let ix = system_instruction::create_account(
        &config.signers[0].pubkey(),
        &program_id.pubkey(),
        minimum_balance.max(1),
        program_data.len() as u64,
        &bpf_loader::id(),
    );
    let message = Message::new(&[ix]);
    let mut create_account_tx = Transaction::new_unsigned(message);
    create_account_tx.try_sign(&[config.signers[0], &program_id], blockhash)?;
    messages.push(&create_account_tx.message);
    let signers = [config.signers[0], &program_id];
    let mut write_transactions = vec![];
    for (chunk, i) in program_data.chunks(DATA_CHUNK_SIZE).zip(0..) {
        let instruction = loader_instruction::write(
            &program_id.pubkey(),
            &bpf_loader::id(),
            (i * DATA_CHUNK_SIZE) as u32,
            chunk.to_vec(),
        );
        let message = Message::new_with_payer(&[instruction], Some(&signers[0].pubkey()));
        let mut tx = Transaction::new_unsigned(message);
        tx.try_sign(&signers, blockhash)?;
        write_transactions.push(tx);
    }
    for transaction in write_transactions.iter() {
        messages.push(&transaction.message);
    }

    let instruction = loader_instruction::finalize(&program_id.pubkey(), &bpf_loader::id());
    let message = Message::new_with_payer(&[instruction], Some(&signers[0].pubkey()));
    let mut finalize_tx = Transaction::new_unsigned(message);
    finalize_tx.try_sign(&signers, blockhash)?;
    messages.push(&finalize_tx.message);

    check_account_for_multiple_fees(
        rpc_client,
        &config.signers[0].pubkey(),
        &fee_calculator,
        &messages,
    )?;

    trace!("Creating program account");
    let result = rpc_client.send_and_confirm_transaction_with_spinner(&create_account_tx);
    log_instruction_custom_error::<SystemError>(result, &config).map_err(|_| {
        CliError::DynamicProgramError("Program account allocation failed".to_string())
    })?;

    trace!("Writing program data");
    rpc_client
        .send_and_confirm_transactions_with_spinner(write_transactions, &signers)
        .map_err(|_| {
            CliError::DynamicProgramError("Data writes to program account failed".to_string())
        })?;

    trace!("Finalizing program account");
    rpc_client
        .send_and_confirm_transaction_with_spinner(&finalize_tx)
        .map_err(|e| {
            CliError::DynamicProgramError(format!("Finalizing program account failed: {}", e))
        })?;

    Ok(json!({
        "programId": format!("{}", program_id.pubkey()),
    })
    .to_string())
}

#[allow(clippy::too_many_arguments)]
fn process_pay(
    rpc_client: &RpcClient,
    config: &CliConfig,
    amount: SpendAmount,
    to: &Pubkey,
    timestamp: Option<DateTime<Utc>>,
    timestamp_pubkey: Option<Pubkey>,
    witnesses: &Option<Vec<Pubkey>>,
    cancelable: bool,
    sign_only: bool,
    blockhash_query: &BlockhashQuery,
    nonce_account: Option<Pubkey>,
    nonce_authority: SignerIndex,
) -> ProcessResult {
    check_unique_pubkeys(
        (&config.signers[0].pubkey(), "cli keypair".to_string()),
        (to, "to".to_string()),
    )?;

    let (blockhash, fee_calculator) =
        blockhash_query.get_blockhash_and_fee_calculator(rpc_client)?;

    let cancelable = if cancelable {
        Some(config.signers[0].pubkey())
    } else {
        None
    };

    if timestamp == None && *witnesses == None {
        let nonce_authority = config.signers[nonce_authority];
        let build_message = |lamports| {
            let ix = system_instruction::transfer(&config.signers[0].pubkey(), to, lamports);
            if let Some(nonce_account) = &nonce_account {
                Message::new_with_nonce(vec![ix], None, nonce_account, &nonce_authority.pubkey())
            } else {
                Message::new(&[ix])
            }
        };

        let (message, _) = resolve_spend_tx_and_check_account_balance(
            rpc_client,
            sign_only,
            amount,
            &fee_calculator,
            &config.signers[0].pubkey(),
            build_message,
        )?;
        let mut tx = Transaction::new_unsigned(message);

        if sign_only {
            tx.try_partial_sign(&config.signers, blockhash)?;
            return_signers(&tx, &config)
        } else {
            if let Some(nonce_account) = &nonce_account {
                let nonce_account = rpc_client.get_account(nonce_account)?;
                check_nonce_account(&nonce_account, &nonce_authority.pubkey(), &blockhash)?;
            }

            tx.try_sign(&config.signers, blockhash)?;
            let result = rpc_client.send_and_confirm_transaction_with_spinner(&tx);
            log_instruction_custom_error::<SystemError>(result, &config)
        }
    } else if *witnesses == None {
        let dt = timestamp.unwrap();
        let dt_pubkey = match timestamp_pubkey {
            Some(pubkey) => pubkey,
            None => config.signers[0].pubkey(),
        };

        let contract_state = Keypair::new();

        let build_message = |lamports| {
            // Initializing contract
            let ixs = budget_instruction::on_date(
                &config.signers[0].pubkey(),
                to,
                &contract_state.pubkey(),
                dt,
                &dt_pubkey,
                cancelable,
                lamports,
            );
            Message::new(&ixs)
        };
        let (message, _) = resolve_spend_tx_and_check_account_balance(
            rpc_client,
            sign_only,
            amount,
            &fee_calculator,
            &config.signers[0].pubkey(),
            build_message,
        )?;
        let mut tx = Transaction::new_unsigned(message);
        if sign_only {
            tx.try_partial_sign(&[config.signers[0], &contract_state], blockhash)?;
            return_signers(&tx, &config)
        } else {
            tx.try_sign(&[config.signers[0], &contract_state], blockhash)?;
            let result = rpc_client.send_and_confirm_transaction_with_spinner(&tx);
            let signature = log_instruction_custom_error::<BudgetError>(result, &config)?;
            Ok(json!({
                "signature": signature,
                "processId": format!("{}", contract_state.pubkey()),
            })
            .to_string())
        }
    } else if timestamp == None {
        let witness = if let Some(ref witness_vec) = *witnesses {
            witness_vec[0]
        } else {
            return Err(CliError::BadParameter(
                "Could not parse required signature pubkey(s)".to_string(),
            )
            .into());
        };

        let contract_state = Keypair::new();

        let build_message = |lamports| {
            // Initializing contract
            let ixs = budget_instruction::when_signed(
                &config.signers[0].pubkey(),
                to,
                &contract_state.pubkey(),
                &witness,
                cancelable,
                lamports,
            );
            Message::new(&ixs)
        };
        let (message, _) = resolve_spend_tx_and_check_account_balance(
            rpc_client,
            sign_only,
            amount,
            &fee_calculator,
            &config.signers[0].pubkey(),
            build_message,
        )?;
        let mut tx = Transaction::new_unsigned(message);
        if sign_only {
            tx.try_partial_sign(&[config.signers[0], &contract_state], blockhash)?;
            return_signers(&tx, &config)
        } else {
            tx.try_sign(&[config.signers[0], &contract_state], blockhash)?;
            let result = rpc_client.send_and_confirm_transaction_with_spinner(&tx);
            let signature = log_instruction_custom_error::<BudgetError>(result, &config)?;
            Ok(json!({
                "signature": signature,
                "processId": format!("{}", contract_state.pubkey()),
            })
            .to_string())
        }
    } else {
        Ok("Combo transactions not yet handled".to_string())
    }
}

fn process_cancel(rpc_client: &RpcClient, config: &CliConfig, pubkey: &Pubkey) -> ProcessResult {
    let (blockhash, fee_calculator) = rpc_client.get_recent_blockhash()?;
    let ix = budget_instruction::apply_signature(
        &config.signers[0].pubkey(),
        pubkey,
        &config.signers[0].pubkey(),
    );
    let message = Message::new(&[ix]);
    let mut tx = Transaction::new_unsigned(message);
    tx.try_sign(&config.signers, blockhash)?;
    check_account_for_fee(
        rpc_client,
        &config.signers[0].pubkey(),
        &fee_calculator,
        &tx.message,
    )?;
    let result = rpc_client.send_and_confirm_transaction_with_spinner(&tx);
    log_instruction_custom_error::<BudgetError>(result, &config)
}

fn process_time_elapsed(
    rpc_client: &RpcClient,
    config: &CliConfig,
    to: &Pubkey,
    pubkey: &Pubkey,
    dt: DateTime<Utc>,
) -> ProcessResult {
    let (blockhash, fee_calculator) = rpc_client.get_recent_blockhash()?;

    let ix = budget_instruction::apply_timestamp(&config.signers[0].pubkey(), pubkey, to, dt);
    let message = Message::new(&[ix]);
    let mut tx = Transaction::new_unsigned(message);
    tx.try_sign(&config.signers, blockhash)?;
    check_account_for_fee(
        rpc_client,
        &config.signers[0].pubkey(),
        &fee_calculator,
        &tx.message,
    )?;
    let result = rpc_client.send_and_confirm_transaction_with_spinner(&tx);
    log_instruction_custom_error::<BudgetError>(result, &config)
}

#[allow(clippy::too_many_arguments)]
fn process_transfer(
    rpc_client: &RpcClient,
    config: &CliConfig,
    amount: SpendAmount,
    to: &Pubkey,
    from: SignerIndex,
    sign_only: bool,
    no_wait: bool,
    blockhash_query: &BlockhashQuery,
    nonce_account: Option<&Pubkey>,
    nonce_authority: SignerIndex,
    fee_payer: SignerIndex,
) -> ProcessResult {
    let from = config.signers[from];

    check_unique_pubkeys(
        (&from.pubkey(), "cli keypair".to_string()),
        (to, "to".to_string()),
    )?;

    let (recent_blockhash, fee_calculator) =
        blockhash_query.get_blockhash_and_fee_calculator(rpc_client)?;

    let nonce_authority = config.signers[nonce_authority];
    let fee_payer = config.signers[fee_payer];

    let build_message = |lamports| {
        let ixs = vec![system_instruction::transfer(&from.pubkey(), to, lamports)];

        if let Some(nonce_account) = &nonce_account {
            Message::new_with_nonce(
                ixs,
                Some(&fee_payer.pubkey()),
                nonce_account,
                &nonce_authority.pubkey(),
            )
        } else {
            Message::new_with_payer(&ixs, Some(&fee_payer.pubkey()))
        }
    };

    let (message, _) = resolve_spend_tx_and_check_account_balances(
        rpc_client,
        sign_only,
        amount,
        &fee_calculator,
        &from.pubkey(),
        &fee_payer.pubkey(),
        build_message,
    )?;
    let mut tx = Transaction::new_unsigned(message);

    if sign_only {
        tx.try_partial_sign(&config.signers, recent_blockhash)?;
        return_signers(&tx, &config)
    } else {
        if let Some(nonce_account) = &nonce_account {
            let nonce_account = rpc_client.get_account(nonce_account)?;
            check_nonce_account(&nonce_account, &nonce_authority.pubkey(), &recent_blockhash)?;
        }

        tx.try_sign(&config.signers, recent_blockhash)?;
        if let Some(nonce_account) = &nonce_account {
            let nonce_account = rpc_client.get_account(nonce_account)?;
            check_nonce_account(&nonce_account, &nonce_authority.pubkey(), &recent_blockhash)?;
        }
        let result = if no_wait {
            rpc_client.send_transaction(&tx)
        } else {
            rpc_client.send_and_confirm_transaction_with_spinner(&tx)
        };
        log_instruction_custom_error::<SystemError>(result, &config)
    }
}

fn process_witness(
    rpc_client: &RpcClient,
    config: &CliConfig,
    to: &Pubkey,
    pubkey: &Pubkey,
) -> ProcessResult {
    let (blockhash, fee_calculator) = rpc_client.get_recent_blockhash()?;

    let ix = budget_instruction::apply_signature(&config.signers[0].pubkey(), pubkey, to);
    let message = Message::new(&[ix]);
    let mut tx = Transaction::new_unsigned(message);
    tx.try_sign(&config.signers, blockhash)?;
    check_account_for_fee(
        rpc_client,
        &config.signers[0].pubkey(),
        &fee_calculator,
        &tx.message,
    )?;
    let result = rpc_client.send_and_confirm_transaction_with_spinner(&tx);
    log_instruction_custom_error::<BudgetError>(result, &config)
}

pub fn process_command(config: &CliConfig) -> ProcessResult {
    if config.verbose && config.output_format == OutputFormat::Display {
        println_name_value("RPC URL:", &config.json_rpc_url);
        println_name_value("Default Signer Path:", &config.keypair_path);
        if config.keypair_path.starts_with("usb://") {
            println_name_value("Pubkey:", &format!("{:?}", config.pubkey()?));
        }
    }

    let mut _rpc_client;
    let rpc_client = if config.rpc_client.is_none() {
        _rpc_client = RpcClient::new(config.json_rpc_url.to_string());
        &_rpc_client
    } else {
        // Primarily for testing
        config.rpc_client.as_ref().unwrap()
    };

    match &config.command {
        // Cluster Query Commands
        // Get address of this client
        CliCommand::Address => Ok(format!("{}", config.pubkey()?)),

        // Return software version of solana-cli and cluster entrypoint node
        CliCommand::Catchup {
            node_pubkey,
            node_json_rpc_url,
            commitment_config,
            follow,
        } => process_catchup(
            &rpc_client,
            node_pubkey,
            node_json_rpc_url,
            *commitment_config,
            *follow,
        ),
        CliCommand::ClusterDate => process_cluster_date(&rpc_client, config),
        CliCommand::ClusterVersion => process_cluster_version(&rpc_client),
        CliCommand::CreateAddressWithSeed {
            from_pubkey,
            seed,
            program_id,
        } => process_create_address_with_seed(config, from_pubkey.as_ref(), &seed, &program_id),
        CliCommand::Fees => process_fees(&rpc_client),
        CliCommand::GetBlockTime { slot } => process_get_block_time(&rpc_client, config, *slot),
        CliCommand::GetGenesisHash => process_get_genesis_hash(&rpc_client),
        CliCommand::GetEpochInfo { commitment_config } => {
            process_get_epoch_info(&rpc_client, config, *commitment_config)
        }
        CliCommand::GetEpoch { commitment_config } => {
            process_get_epoch(&rpc_client, *commitment_config)
        }
        CliCommand::GetSlot { commitment_config } => {
            process_get_slot(&rpc_client, *commitment_config)
        }
        CliCommand::LargestAccounts {
            commitment_config,
            filter,
        } => process_largest_accounts(&rpc_client, config, *commitment_config, filter.clone()),
        CliCommand::Supply {
            commitment_config,
            print_accounts,
        } => process_supply(&rpc_client, config, *commitment_config, *print_accounts),
        CliCommand::TotalSupply { commitment_config } => {
            process_total_supply(&rpc_client, *commitment_config)
        }
        CliCommand::GetTransactionCount { commitment_config } => {
            process_get_transaction_count(&rpc_client, *commitment_config)
        }
        CliCommand::LeaderSchedule => process_leader_schedule(&rpc_client),
        CliCommand::LiveSlots => process_live_slots(&config.websocket_url),
        CliCommand::Ping {
            lamports,
            interval,
            count,
            timeout,
            commitment_config,
        } => process_ping(
            &rpc_client,
            config,
            *lamports,
            interval,
            count,
            timeout,
            *commitment_config,
        ),
        CliCommand::ShowBlockProduction { epoch, slot_limit } => {
            process_show_block_production(&rpc_client, config, *epoch, *slot_limit)
        }
        CliCommand::ShowGossip => process_show_gossip(&rpc_client),
        CliCommand::ShowStakes {
            use_lamports_unit,
            vote_account_pubkeys,
        } => process_show_stakes(
            &rpc_client,
            config,
            *use_lamports_unit,
            vote_account_pubkeys.as_deref(),
        ),
        CliCommand::ShowValidators {
            use_lamports_unit,
            commitment_config,
        } => process_show_validators(&rpc_client, config, *use_lamports_unit, *commitment_config),
        CliCommand::TransactionHistory {
            address,
            end_slot,
            slot_limit,
        } => process_transaction_history(&rpc_client, address, *end_slot, *slot_limit),

        // Nonce Commands

        // Assign authority to nonce account
        CliCommand::AuthorizeNonceAccount {
            nonce_account,
            nonce_authority,
            new_authority,
        } => process_authorize_nonce_account(
            &rpc_client,
            config,
            nonce_account,
            *nonce_authority,
            new_authority,
        ),
        // Create nonce account
        CliCommand::CreateNonceAccount {
            nonce_account,
            seed,
            nonce_authority,
            amount,
        } => process_create_nonce_account(
            &rpc_client,
            config,
            *nonce_account,
            seed.clone(),
            *nonce_authority,
            *amount,
        ),
        // Get the current nonce
        CliCommand::GetNonce(nonce_account_pubkey) => {
            process_get_nonce(&rpc_client, &nonce_account_pubkey)
        }
        // Get a new nonce
        CliCommand::NewNonce {
            nonce_account,
            nonce_authority,
        } => process_new_nonce(&rpc_client, config, nonce_account, *nonce_authority),
        // Show the contents of a nonce account
        CliCommand::ShowNonceAccount {
            nonce_account_pubkey,
            use_lamports_unit,
        } => process_show_nonce_account(
            &rpc_client,
            config,
            &nonce_account_pubkey,
            *use_lamports_unit,
        ),
        // Withdraw lamports from a nonce account
        CliCommand::WithdrawFromNonceAccount {
            nonce_account,
            nonce_authority,
            destination_account_pubkey,
            lamports,
        } => process_withdraw_from_nonce_account(
            &rpc_client,
            config,
            &nonce_account,
            *nonce_authority,
            &destination_account_pubkey,
            *lamports,
        ),

        // Program Deployment

        // Deploy a custom program to the chain
        CliCommand::Deploy(ref program_location) => {
            process_deploy(&rpc_client, config, program_location)
        }

        // Stake Commands

        // Create stake account
        CliCommand::CreateStakeAccount {
            stake_account,
            seed,
            staker,
            withdrawer,
            lockup,
            amount,
            sign_only,
            blockhash_query,
            ref nonce_account,
            nonce_authority,
            fee_payer,
            from,
        } => process_create_stake_account(
            &rpc_client,
            config,
            *stake_account,
            seed,
            staker,
            withdrawer,
            lockup,
            *amount,
            *sign_only,
            blockhash_query,
            nonce_account.as_ref(),
            *nonce_authority,
            *fee_payer,
            *from,
        ),
        CliCommand::DeactivateStake {
            stake_account_pubkey,
            stake_authority,
            sign_only,
            blockhash_query,
            nonce_account,
            nonce_authority,
            fee_payer,
        } => process_deactivate_stake_account(
            &rpc_client,
            config,
            &stake_account_pubkey,
            *stake_authority,
            *sign_only,
            blockhash_query,
            *nonce_account,
            *nonce_authority,
            *fee_payer,
        ),
        CliCommand::DelegateStake {
            stake_account_pubkey,
            vote_account_pubkey,
            stake_authority,
            force,
            sign_only,
            blockhash_query,
            nonce_account,
            nonce_authority,
            fee_payer,
        } => process_delegate_stake(
            &rpc_client,
            config,
            &stake_account_pubkey,
            &vote_account_pubkey,
            *stake_authority,
            *force,
            *sign_only,
            blockhash_query,
            *nonce_account,
            *nonce_authority,
            *fee_payer,
        ),
        CliCommand::SplitStake {
            stake_account_pubkey,
            stake_authority,
            sign_only,
            blockhash_query,
            nonce_account,
            nonce_authority,
            split_stake_account,
            seed,
            lamports,
            fee_payer,
        } => process_split_stake(
            &rpc_client,
            config,
            &stake_account_pubkey,
            *stake_authority,
            *sign_only,
            blockhash_query,
            *nonce_account,
            *nonce_authority,
            *split_stake_account,
            seed,
            *lamports,
            *fee_payer,
        ),
        CliCommand::ShowStakeAccount {
            pubkey: stake_account_pubkey,
            use_lamports_unit,
        } => process_show_stake_account(
            &rpc_client,
            config,
            &stake_account_pubkey,
            *use_lamports_unit,
        ),
        CliCommand::ShowStakeHistory { use_lamports_unit } => {
            process_show_stake_history(&rpc_client, config, *use_lamports_unit)
        }
        CliCommand::StakeAuthorize {
            stake_account_pubkey,
            ref new_authorizations,
            sign_only,
            blockhash_query,
            nonce_account,
            nonce_authority,
            fee_payer,
        } => process_stake_authorize(
            &rpc_client,
            config,
            &stake_account_pubkey,
            new_authorizations,
            *sign_only,
            blockhash_query,
            *nonce_account,
            *nonce_authority,
            *fee_payer,
        ),
        CliCommand::StakeSetLockup {
            stake_account_pubkey,
            mut lockup,
            custodian,
            sign_only,
            blockhash_query,
            nonce_account,
            nonce_authority,
            fee_payer,
        } => process_stake_set_lockup(
            &rpc_client,
            config,
            &stake_account_pubkey,
            &mut lockup,
            *custodian,
            *sign_only,
            blockhash_query,
            *nonce_account,
            *nonce_authority,
            *fee_payer,
        ),
        CliCommand::WithdrawStake {
            stake_account_pubkey,
            destination_account_pubkey,
            lamports,
            withdraw_authority,
            custodian,
            sign_only,
            blockhash_query,
            ref nonce_account,
            nonce_authority,
            fee_payer,
        } => process_withdraw_stake(
            &rpc_client,
            config,
            &stake_account_pubkey,
            &destination_account_pubkey,
            *lamports,
            *withdraw_authority,
            *custodian,
            *sign_only,
            blockhash_query,
            nonce_account.as_ref(),
            *nonce_authority,
            *fee_payer,
        ),

        // Storage Commands

        // Create storage account
        CliCommand::CreateStorageAccount {
            account_owner,
            storage_account,
            account_type,
        } => process_create_storage_account(
            &rpc_client,
            config,
            *storage_account,
            &account_owner,
            *account_type,
        ),
        CliCommand::ClaimStorageReward {
            node_account_pubkey,
            storage_account_pubkey,
        } => process_claim_storage_reward(
            &rpc_client,
            config,
            node_account_pubkey,
            &storage_account_pubkey,
        ),
        CliCommand::ShowStorageAccount(storage_account_pubkey) => {
            process_show_storage_account(&rpc_client, config, &storage_account_pubkey)
        }

        // Validator Info Commands

        // Return all or single validator info
        CliCommand::GetValidatorInfo(info_pubkey) => {
            process_get_validator_info(&rpc_client, config, *info_pubkey)
        }
        // Publish validator info
        CliCommand::SetValidatorInfo {
            validator_info,
            force_keybase,
            info_pubkey,
        } => process_set_validator_info(
            &rpc_client,
            config,
            &validator_info,
            *force_keybase,
            *info_pubkey,
        ),

        // Vote Commands

        // Create vote account
        CliCommand::CreateVoteAccount {
            seed,
            identity_account,
            authorized_voter,
            authorized_withdrawer,
            commission,
        } => process_create_vote_account(
            &rpc_client,
            config,
            seed,
            *identity_account,
            authorized_voter,
            authorized_withdrawer,
            *commission,
        ),
        CliCommand::ShowVoteAccount {
            pubkey: vote_account_pubkey,
            use_lamports_unit,
            commitment_config,
        } => process_show_vote_account(
            &rpc_client,
            config,
            &vote_account_pubkey,
            *use_lamports_unit,
            *commitment_config,
        ),
        CliCommand::WithdrawFromVoteAccount {
            vote_account_pubkey,
            withdraw_authority,
            lamports,
            destination_account_pubkey,
        } => process_withdraw_from_vote_account(
            &rpc_client,
            config,
            vote_account_pubkey,
            *withdraw_authority,
            *lamports,
            destination_account_pubkey,
        ),
        CliCommand::VoteAuthorize {
            vote_account_pubkey,
            new_authorized_pubkey,
            vote_authorize,
        } => process_vote_authorize(
            &rpc_client,
            config,
            &vote_account_pubkey,
            &new_authorized_pubkey,
            *vote_authorize,
        ),
        CliCommand::VoteUpdateValidator {
            vote_account_pubkey,
            new_identity_account,
        } => process_vote_update_validator(
            &rpc_client,
            config,
            &vote_account_pubkey,
            *new_identity_account,
        ),

        // Wallet Commands

        // Request an airdrop from Solana Faucet;
        CliCommand::Airdrop {
            faucet_host,
            faucet_port,
            pubkey,
            lamports,
        } => {
            let faucet_addr = SocketAddr::new(
                faucet_host.unwrap_or_else(|| {
                    let faucet_host = Url::parse(&config.json_rpc_url)
                        .unwrap()
                        .host()
                        .unwrap()
                        .to_string();
                    solana_net_utils::parse_host(&faucet_host).unwrap_or_else(|err| {
                        panic!("Unable to resolve {}: {}", faucet_host, err);
                    })
                }),
                *faucet_port,
            );

            process_airdrop(&rpc_client, config, &faucet_addr, pubkey, *lamports)
        }
        // Check client balance
        CliCommand::Balance {
            pubkey,
            use_lamports_unit,
            commitment_config,
        } => process_balance(
            &rpc_client,
            config,
            &pubkey,
            *use_lamports_unit,
            *commitment_config,
        ),
        // Cancel a contract by contract Pubkey
        CliCommand::Cancel(pubkey) => process_cancel(&rpc_client, config, &pubkey),
        // Confirm the last client transaction by signature
        CliCommand::Confirm(signature) => process_confirm(&rpc_client, config, signature),
        CliCommand::DecodeTransaction(transaction) => process_decode_transaction(transaction),
        // If client has positive balance, pay lamports to another address
        CliCommand::Pay(PayCommand {
            amount,
            to,
            timestamp,
            timestamp_pubkey,
            ref witnesses,
            cancelable,
            sign_only,
            blockhash_query,
            nonce_account,
            nonce_authority,
        }) => process_pay(
            &rpc_client,
            config,
            *amount,
            &to,
            *timestamp,
            *timestamp_pubkey,
            witnesses,
            *cancelable,
            *sign_only,
            blockhash_query,
            *nonce_account,
            *nonce_authority,
        ),
        CliCommand::ResolveSigner(path) => {
            if let Some(path) = path {
                Ok(path.to_string())
            } else {
                Ok("Signer is valid".to_string())
            }
        }
        CliCommand::ShowAccount {
            pubkey,
            output_file,
            use_lamports_unit,
        } => process_show_account(
            &rpc_client,
            config,
            &pubkey,
            &output_file,
            *use_lamports_unit,
        ),
        // Apply time elapsed to contract
        CliCommand::TimeElapsed(to, pubkey, dt) => {
            process_time_elapsed(&rpc_client, config, &to, &pubkey, *dt)
        }
        CliCommand::Transfer {
            amount,
            to,
            from,
            sign_only,
            no_wait,
            ref blockhash_query,
            ref nonce_account,
            nonce_authority,
            fee_payer,
        } => process_transfer(
            &rpc_client,
            config,
            *amount,
            to,
            *from,
            *sign_only,
            *no_wait,
            blockhash_query,
            nonce_account.as_ref(),
            *nonce_authority,
            *fee_payer,
        ),
        // Apply witness signature to contract
        CliCommand::Witness(to, pubkey) => process_witness(&rpc_client, config, &to, &pubkey),
    }
}

// Quick and dirty Keypair that assumes the client will do retries but not update the
// blockhash. If the client updates the blockhash, the signature will be invalid.
struct FaucetKeypair {
    transaction: Transaction,
}

impl FaucetKeypair {
    fn new_keypair(
        faucet_addr: &SocketAddr,
        to_pubkey: &Pubkey,
        lamports: u64,
        blockhash: Hash,
    ) -> Result<Self, Box<dyn error::Error>> {
        let transaction = request_airdrop_transaction(faucet_addr, to_pubkey, lamports, blockhash)?;
        Ok(Self { transaction })
    }

    fn airdrop_transaction(&self) -> Transaction {
        self.transaction.clone()
    }
}

impl Signer for FaucetKeypair {
    /// Return the public key of the keypair used to sign votes
    fn pubkey(&self) -> Pubkey {
        self.transaction.message().account_keys[0]
    }

    fn try_pubkey(&self) -> Result<Pubkey, SignerError> {
        Ok(self.pubkey())
    }

    fn sign_message(&self, _msg: &[u8]) -> Signature {
        self.transaction.signatures[0]
    }

    fn try_sign_message(&self, message: &[u8]) -> Result<Signature, SignerError> {
        Ok(self.sign_message(message))
    }
}

pub fn request_and_confirm_airdrop(
    rpc_client: &RpcClient,
    faucet_addr: &SocketAddr,
    to_pubkey: &Pubkey,
    lamports: u64,
    config: &CliConfig,
) -> ProcessResult {
    let (blockhash, _fee_calculator) = rpc_client.get_recent_blockhash()?;
    let keypair = {
        let mut retries = 5;
        loop {
            let result = FaucetKeypair::new_keypair(faucet_addr, to_pubkey, lamports, blockhash);
            if result.is_ok() || retries == 0 {
                break result;
            }
            retries -= 1;
            sleep(Duration::from_secs(1));
        }
    }?;
    let tx = keypair.airdrop_transaction();
    let result = rpc_client.send_and_confirm_transaction_with_spinner(&tx);
    log_instruction_custom_error::<SystemError>(result, &config)
}

pub fn log_instruction_custom_error<E>(
    result: ClientResult<Signature>,
    config: &CliConfig,
) -> ProcessResult
where
    E: 'static + std::error::Error + DecodeError<E> + FromPrimitive,
{
    match result {
        Err(err) => {
            if let ClientErrorKind::TransactionError(TransactionError::InstructionError(
                _,
                InstructionError::Custom(code),
            )) = err.kind()
            {
                if let Some(specific_error) = E::decode_custom_error_to_enum(*code) {
                    return Err(specific_error.into());
                }
            }
            Err(err.into())
        }
        Ok(sig) => {
            let signature = CliSignature {
                signature: sig.clone().to_string(),
            };
            Ok(config.output_format.formatted_string(&signature))
        }
    }
}

pub(crate) fn build_balance_message(
    lamports: u64,
    use_lamports_unit: bool,
    show_unit: bool,
) -> String {
    if use_lamports_unit {
        let ess = if lamports == 1 { "" } else { "s" };
        let unit = if show_unit {
            format!(" lamport{}", ess)
        } else {
            "".to_string()
        };
        format!("{:?}{}", lamports, unit)
    } else {
        let sol = lamports_to_sol(lamports);
        let sol_str = format!("{:.9}", sol);
        let pretty_sol = sol_str.trim_end_matches('0').trim_end_matches('.');
        let unit = if show_unit { " SOL" } else { "" };
        format!("{}{}", pretty_sol, unit)
    }
}

pub fn app<'ab, 'v>(name: &str, about: &'ab str, version: &'v str) -> App<'ab, 'v> {
    App::new(name)
        .about(about)
        .version(version)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("address")
                .about("Get your public key")
                .arg(
                    Arg::with_name("confirm_key")
                        .long("confirm-key")
                        .takes_value(false)
                        .help("Confirm key on device; only relevant if using remote wallet"),
                ),
        )
        .cluster_query_subcommands()
        .nonce_subcommands()
        .stake_subcommands()
        .storage_subcommands()
        .subcommand(
            SubCommand::with_name("airdrop")
                .about("Request lamports")
                .arg(
                    Arg::with_name("faucet_host")
                        .long("faucet-host")
                        .value_name("URL")
                        .takes_value(true)
                        .help("Faucet host to use [default: the --url host]"),
                )
                .arg(
                    Arg::with_name("faucet_port")
                        .long("faucet-port")
                        .value_name("PORT_NUMBER")
                        .takes_value(true)
                        .default_value(solana_faucet::faucet::FAUCET_PORT_STR)
                        .help("Faucet port to use"),
                )
                .arg(
                    Arg::with_name("amount")
                        .index(1)
                        .value_name("AMOUNT")
                        .takes_value(true)
                        .validator(is_amount)
                        .required(true)
                        .help("The airdrop amount to request, in SOL"),
                )
                .arg(
                    pubkey!(Arg::with_name("to")
                        .index(2)
                        .value_name("RECIPIENT_ADDRESS"),
                        "The account address of airdrop recipient. "),
                ),
        )
        .subcommand(
            SubCommand::with_name("balance")
                .about("Get your balance")
                .arg(
                    pubkey!(Arg::with_name("pubkey")
                        .index(1)
                        .value_name("ACCOUNT_ADDRESS"),
                        "The account address of the balance to check. ")
                )
                .arg(
                    Arg::with_name("lamports")
                        .long("lamports")
                        .takes_value(false)
                        .help("Display balance in lamports instead of SOL"),
                )
                .arg(commitment_arg_with_default("max")),
        )
        .subcommand(
            SubCommand::with_name("cancel")
                .about("Cancel a transfer")
                .arg(
                    Arg::with_name("process_id")
                        .index(1)
                        .value_name("ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .validator(is_pubkey)
                        .help("The account address of the transfer to cancel"),
                ),
        )
        .subcommand(
            SubCommand::with_name("confirm")
                .about("Confirm transaction by signature")
                .arg(
                    Arg::with_name("signature")
                        .index(1)
                        .value_name("TRANSACTION_SIGNATURE")
                        .takes_value(true)
                        .required(true)
                        .help("The transaction signature to confirm"),
                ),
        )
        .subcommand(
            SubCommand::with_name("decode-transaction")
                .about("Decode a base-58 binary transaction")
                .arg(
                    Arg::with_name("base58_transaction")
                        .index(1)
                        .value_name("BASE58_TRANSACTION")
                        .takes_value(true)
                        .required(true)
                        .help("The transaction to decode"),
                ),
        )
        .subcommand(
            SubCommand::with_name("create-address-with-seed")
                .about("Generate a derived account address with a seed")
                .arg(
                    Arg::with_name("seed")
                        .index(1)
                        .value_name("SEED_STRING")
                        .takes_value(true)
                        .required(true)
                        .help("The seed.  Must not take more than 32 bytes to encode as utf-8"),
                )
                .arg(
                    Arg::with_name("program_id")
                        .index(2)
                        .value_name("PROGRAM_ID")
                        .takes_value(true)
                        .required(true)
                        .help(
                            "The program_id that the address will ultimately be used for, \n\
                             or one of NONCE, STAKE, STORAGE, and VOTE keywords",
                        ),
                )
                .arg(
                    pubkey!(Arg::with_name("from")
                        .long("from")
                        .value_name("FROM_PUBKEY")
                        .required(false),
                        "From (base) key, [default: cli config keypair]. "),
                ),
        )
        .subcommand(
            SubCommand::with_name("deploy")
                .about("Deploy a program")
                .arg(
                    Arg::with_name("program_location")
                        .index(1)
                        .value_name("PROGRAM_FILEPATH")
                        .takes_value(true)
                        .required(true)
                        .help("/path/to/program.o"),
                ),
        )
        .subcommand(
            SubCommand::with_name("pay")
                .about("Send a payment")
                .arg(
                    pubkey!(Arg::with_name("to")
                        .index(1)
                        .value_name("RECIPIENT_ADDRESS")
                        .required(true),
                        "The account address of recipient. "),
                )
                .arg(
                    Arg::with_name("amount")
                        .index(2)
                        .value_name("AMOUNT")
                        .takes_value(true)
                        .validator(is_amount_or_all)
                        .required(true)
                        .help("The amount to send, in SOL; accepts keyword ALL"),
                )
                .arg(
                    Arg::with_name("timestamp")
                        .long("after")
                        .value_name("DATETIME")
                        .takes_value(true)
                        .help("A timestamp after which transaction will execute"),
                )
                .arg(
                    Arg::with_name("timestamp_pubkey")
                        .long("require-timestamp-from")
                        .value_name("PUBKEY")
                        .takes_value(true)
                        .requires("timestamp")
                        .validator(is_pubkey)
                        .help("Require timestamp from this third party"),
                )
                .arg(
                    Arg::with_name("witness")
                        .long("require-signature-from")
                        .value_name("PUBKEY")
                        .takes_value(true)
                        .multiple(true)
                        .use_delimiter(true)
                        .validator(is_pubkey)
                        .help("Any third party signatures required to unlock the lamports"),
                )
                .arg(
                    Arg::with_name("cancelable")
                        .long("cancelable")
                        .takes_value(false),
                )
                .offline_args()
                .arg(nonce_arg())
                .arg(nonce_authority_arg()),
        )
        .subcommand(
            SubCommand::with_name("resolve-signer")
                .about("Checks that a signer is valid, and returns its specific path; useful for signers that may be specified generally, eg. usb://ledger")
                .arg(
                    Arg::with_name("signer")
                        .index(1)
                        .value_name("SIGNER_KEYPAIR")
                        .takes_value(true)
                        .required(true)
                        .validator(is_valid_signer)
                        .help("The signer path to resolve")
                )
        )
        .subcommand(
            SubCommand::with_name("send-signature")
                .about("Send a signature to authorize a transfer")
                .arg(
                    pubkey!(Arg::with_name("to")
                        .index(1)
                        .value_name("RECIPIENT_ADDRESS")
                        .required(true),
                        "The account address of recipient. "),
                )
                .arg(
                    Arg::with_name("process_id")
                        .index(2)
                        .value_name("ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("The account address of the transfer to authorize"),
                ),
        )
        .subcommand(
            SubCommand::with_name("send-timestamp")
                .about("Send a timestamp to unlock a transfer")
                .arg(
                    pubkey!(Arg::with_name("to")
                        .index(1)
                        .value_name("RECIPIENT_ADDRESS")
                        .required(true),
                        "The account address of recipient. "),
                )
                .arg(
                    Arg::with_name("process_id")
                        .index(2)
                        .value_name("ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("The account address of the transfer to unlock"),
                )
                .arg(
                    Arg::with_name("datetime")
                        .long("date")
                        .value_name("DATETIME")
                        .takes_value(true)
                        .help("Optional arbitrary timestamp to apply"),
                ),
        )
        .subcommand(
            SubCommand::with_name("transfer")
                .about("Transfer funds between system accounts")
                .arg(
                    pubkey!(Arg::with_name("to")
                        .index(1)
                        .value_name("RECIPIENT_ADDRESS")
                        .required(true),
                        "The account address of recipient. "),
                )
                .arg(
                    Arg::with_name("amount")
                        .index(2)
                        .value_name("AMOUNT")
                        .takes_value(true)
                        .validator(is_amount_or_all)
                        .required(true)
                        .help("The amount to send, in SOL; accepts keyword ALL"),
                )
                .arg(
                    pubkey!(Arg::with_name("from")
                        .long("from")
                        .value_name("FROM_ADDRESS"),
                        "Source account of funds (if different from client local account). "),
                )
                .arg(
                    Arg::with_name("no_wait")
                        .long("no-wait")
                        .takes_value(false)
                        .help("Return signature immediately after submitting the transaction, instead of waiting for confirmations"),
                )
                .offline_args()
                .arg(nonce_arg())
                .arg(nonce_authority_arg())
                .arg(fee_payer_arg()),
        )
        .subcommand(
            SubCommand::with_name("account")
                .about("Show the contents of an account")
                .alias("account")
                .arg(
                    pubkey!(Arg::with_name("account_pubkey")
                        .index(1)
                        .value_name("ACCOUNT_ADDRESS")
                        .required(true),
                        "Account key URI. ")
                )
                .arg(
                    Arg::with_name("output_file")
                        .long("output-file")
                        .short("o")
                        .value_name("FILEPATH")
                        .takes_value(true)
                        .help("Write the account data to this file"),
                )
                .arg(
                    Arg::with_name("lamports")
                        .long("lamports")
                        .takes_value(false)
                        .help("Display balance in lamports instead of SOL"),
                ),
        )
        .validator_info_subcommands()
        .vote_subcommands()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use solana_client::mock_rpc_client_request::SIGNATURE;
    use solana_sdk::{
        pubkey::Pubkey,
        signature::{
            keypair_from_seed, read_keypair_file, write_keypair_file, NullSigner, Presigner,
        },
        transaction::TransactionError,
    };
    use std::path::PathBuf;

    fn make_tmp_path(name: &str) -> String {
        let out_dir = std::env::var("FARF_DIR").unwrap_or_else(|_| "farf".to_string());
        let keypair = Keypair::new();

        let path = format!("{}/tmp/{}-{}", out_dir, name, keypair.pubkey());

        // whack any possible collision
        let _ignored = std::fs::remove_dir_all(&path);
        // whack any possible collision
        let _ignored = std::fs::remove_file(&path);

        path
    }

    #[test]
    fn test_generate_unique_signers() {
        let matches = ArgMatches::default();

        let default_keypair = Keypair::new();
        let default_keypair_file = make_tmp_path("keypair_file");
        write_keypair_file(&default_keypair, &default_keypair_file).unwrap();

        let signer_info =
            generate_unique_signers(vec![], &matches, &default_keypair_file, &mut None).unwrap();
        assert_eq!(signer_info.signers.len(), 0);

        let signer_info =
            generate_unique_signers(vec![None, None], &matches, &default_keypair_file, &mut None)
                .unwrap();
        assert_eq!(signer_info.signers.len(), 1);
        assert_eq!(signer_info.index_of(None), Some(0));
        assert_eq!(signer_info.index_of(Some(Pubkey::new_rand())), None);

        let keypair0 = keypair_from_seed(&[1u8; 32]).unwrap();
        let keypair0_pubkey = keypair0.pubkey();
        let keypair0_clone = keypair_from_seed(&[1u8; 32]).unwrap();
        let keypair0_clone_pubkey = keypair0.pubkey();
        let signers = vec![None, Some(keypair0.into()), Some(keypair0_clone.into())];
        let signer_info =
            generate_unique_signers(signers, &matches, &default_keypair_file, &mut None).unwrap();
        assert_eq!(signer_info.signers.len(), 2);
        assert_eq!(signer_info.index_of(None), Some(0));
        assert_eq!(signer_info.index_of(Some(keypair0_pubkey)), Some(1));
        assert_eq!(signer_info.index_of(Some(keypair0_clone_pubkey)), Some(1));

        let keypair0 = keypair_from_seed(&[1u8; 32]).unwrap();
        let keypair0_pubkey = keypair0.pubkey();
        let keypair0_clone = keypair_from_seed(&[1u8; 32]).unwrap();
        let signers = vec![Some(keypair0.into()), Some(keypair0_clone.into())];
        let signer_info =
            generate_unique_signers(signers, &matches, &default_keypair_file, &mut None).unwrap();
        assert_eq!(signer_info.signers.len(), 1);
        assert_eq!(signer_info.index_of(Some(keypair0_pubkey)), Some(0));

        // Signers with the same pubkey are not distinct
        let keypair0 = keypair_from_seed(&[2u8; 32]).unwrap();
        let keypair0_pubkey = keypair0.pubkey();
        let keypair1 = keypair_from_seed(&[3u8; 32]).unwrap();
        let keypair1_pubkey = keypair1.pubkey();
        let message = vec![0, 1, 2, 3];
        let presigner0 = Presigner::new(&keypair0.pubkey(), &keypair0.sign_message(&message));
        let presigner0_pubkey = presigner0.pubkey();
        let presigner1 = Presigner::new(&keypair1.pubkey(), &keypair1.sign_message(&message));
        let presigner1_pubkey = presigner1.pubkey();
        let signers = vec![
            Some(keypair0.into()),
            Some(presigner0.into()),
            Some(presigner1.into()),
            Some(keypair1.into()),
        ];
        let signer_info =
            generate_unique_signers(signers, &matches, &default_keypair_file, &mut None).unwrap();
        assert_eq!(signer_info.signers.len(), 2);
        assert_eq!(signer_info.index_of(Some(keypair0_pubkey)), Some(0));
        assert_eq!(signer_info.index_of(Some(keypair1_pubkey)), Some(1));
        assert_eq!(signer_info.index_of(Some(presigner0_pubkey)), Some(0));
        assert_eq!(signer_info.index_of(Some(presigner1_pubkey)), Some(1));
    }

    #[test]
    fn test_cli_parse_command() {
        let test_commands = app("test", "desc", "version");

        let pubkey = Pubkey::new_rand();
        let pubkey_string = format!("{}", pubkey);
        let witness0 = Pubkey::new_rand();
        let witness0_string = format!("{}", witness0);
        let witness1 = Pubkey::new_rand();
        let witness1_string = format!("{}", witness1);
        let dt = Utc.ymd(2018, 9, 19).and_hms(17, 30, 59);

        // Test Airdrop Subcommand
        let test_airdrop =
            test_commands
                .clone()
                .get_matches_from(vec!["test", "airdrop", "50", &pubkey_string]);
        assert_eq!(
            parse_command(&test_airdrop, "", &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::Airdrop {
                    faucet_host: None,
                    faucet_port: solana_faucet::faucet::FAUCET_PORT,
                    pubkey: Some(pubkey),
                    lamports: 50_000_000_000,
                },
                signers: vec![],
            }
        );

        // Test Balance Subcommand, incl pubkey and keypair-file inputs
        let default_keypair = Keypair::new();
        let keypair_file = make_tmp_path("keypair_file");
        write_keypair_file(&default_keypair, &keypair_file).unwrap();
        let keypair = read_keypair_file(&keypair_file).unwrap();
        let test_balance = test_commands.clone().get_matches_from(vec![
            "test",
            "balance",
            &keypair.pubkey().to_string(),
        ]);
        assert_eq!(
            parse_command(&test_balance, "", &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::Balance {
                    pubkey: Some(keypair.pubkey()),
                    use_lamports_unit: false,
                    commitment_config: CommitmentConfig::default(),
                },
                signers: vec![],
            }
        );
        let test_balance = test_commands.clone().get_matches_from(vec![
            "test",
            "balance",
            &keypair_file,
            "--lamports",
        ]);
        assert_eq!(
            parse_command(&test_balance, "", &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::Balance {
                    pubkey: Some(keypair.pubkey()),
                    use_lamports_unit: true,
                    commitment_config: CommitmentConfig::default(),
                },
                signers: vec![],
            }
        );
        let test_balance =
            test_commands
                .clone()
                .get_matches_from(vec!["test", "balance", "--lamports"]);
        assert_eq!(
            parse_command(&test_balance, &keypair_file, &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::Balance {
                    pubkey: None,
                    use_lamports_unit: true,
                    commitment_config: CommitmentConfig::default(),
                },
                signers: vec![read_keypair_file(&keypair_file).unwrap().into()],
            }
        );

        // Test Cancel Subcommand
        let test_cancel =
            test_commands
                .clone()
                .get_matches_from(vec!["test", "cancel", &pubkey_string]);
        assert_eq!(
            parse_command(&test_cancel, &keypair_file, &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::Cancel(pubkey),
                signers: vec![read_keypair_file(&keypair_file).unwrap().into()],
            }
        );

        // Test Confirm Subcommand
        let signature = Signature::new(&vec![1; 64]);
        let signature_string = format!("{:?}", signature);
        let test_confirm =
            test_commands
                .clone()
                .get_matches_from(vec!["test", "confirm", &signature_string]);
        assert_eq!(
            parse_command(&test_confirm, "", &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::Confirm(signature),
                signers: vec![],
            }
        );
        let test_bad_signature = test_commands
            .clone()
            .get_matches_from(vec!["test", "confirm", "deadbeef"]);
        assert!(parse_command(&test_bad_signature, "", &mut None).is_err());

        // Test CreateAddressWithSeed
        let from_pubkey = Some(Pubkey::new_rand());
        let from_str = from_pubkey.unwrap().to_string();
        for (name, program_id) in &[
            ("STAKE", solana_stake_program::id()),
            ("VOTE", solana_vote_program::id()),
            ("STORAGE", solana_storage_program::id()),
            ("NONCE", system_program::id()),
        ] {
            let test_create_address_with_seed = test_commands.clone().get_matches_from(vec![
                "test",
                "create-address-with-seed",
                "seed",
                name,
                "--from",
                &from_str,
            ]);
            assert_eq!(
                parse_command(&test_create_address_with_seed, "", &mut None).unwrap(),
                CliCommandInfo {
                    command: CliCommand::CreateAddressWithSeed {
                        from_pubkey,
                        seed: "seed".to_string(),
                        program_id: *program_id
                    },
                    signers: vec![],
                }
            );
        }
        let test_create_address_with_seed = test_commands.clone().get_matches_from(vec![
            "test",
            "create-address-with-seed",
            "seed",
            "STAKE",
        ]);
        assert_eq!(
            parse_command(&test_create_address_with_seed, &keypair_file, &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::CreateAddressWithSeed {
                    from_pubkey: None,
                    seed: "seed".to_string(),
                    program_id: solana_stake_program::id(),
                },
                signers: vec![read_keypair_file(&keypair_file).unwrap().into()],
            }
        );

        // Test Deploy Subcommand
        let test_deploy =
            test_commands
                .clone()
                .get_matches_from(vec!["test", "deploy", "/Users/test/program.o"]);
        assert_eq!(
            parse_command(&test_deploy, &keypair_file, &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::Deploy("/Users/test/program.o".to_string()),
                signers: vec![read_keypair_file(&keypair_file).unwrap().into()],
            }
        );

        // Test ResolveSigner Subcommand, KeypairUrl::Filepath
        let test_resolve_signer =
            test_commands
                .clone()
                .get_matches_from(vec!["test", "resolve-signer", &keypair_file]);
        assert_eq!(
            parse_command(&test_resolve_signer, "", &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::ResolveSigner(Some(keypair_file.clone())),
                signers: vec![],
            }
        );
        // Test ResolveSigner Subcommand, KeypairUrl::Pubkey (Presigner)
        let test_resolve_signer =
            test_commands
                .clone()
                .get_matches_from(vec!["test", "resolve-signer", &pubkey_string]);
        assert_eq!(
            parse_command(&test_resolve_signer, "", &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::ResolveSigner(Some(pubkey.to_string())),
                signers: vec![],
            }
        );

        // Test Simple Pay Subcommand
        let test_pay =
            test_commands
                .clone()
                .get_matches_from(vec!["test", "pay", &pubkey_string, "50"]);
        assert_eq!(
            parse_command(&test_pay, &keypair_file, &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::Pay(PayCommand {
                    amount: SpendAmount::Some(50_000_000_000),
                    to: pubkey,
                    ..PayCommand::default()
                }),
                signers: vec![read_keypair_file(&keypair_file).unwrap().into()],
            }
        );

        // Test Pay Subcommand w/ Witness
        let test_pay_multiple_witnesses = test_commands.clone().get_matches_from(vec![
            "test",
            "pay",
            &pubkey_string,
            "50",
            "--require-signature-from",
            &witness0_string,
            "--require-signature-from",
            &witness1_string,
        ]);
        assert_eq!(
            parse_command(&test_pay_multiple_witnesses, &keypair_file, &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::Pay(PayCommand {
                    amount: SpendAmount::Some(50_000_000_000),
                    to: pubkey,
                    witnesses: Some(vec![witness0, witness1]),
                    ..PayCommand::default()
                }),
                signers: vec![read_keypair_file(&keypair_file).unwrap().into()],
            }
        );
        let test_pay_single_witness = test_commands.clone().get_matches_from(vec![
            "test",
            "pay",
            &pubkey_string,
            "50",
            "--require-signature-from",
            &witness0_string,
        ]);
        assert_eq!(
            parse_command(&test_pay_single_witness, &keypair_file, &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::Pay(PayCommand {
                    amount: SpendAmount::Some(50_000_000_000),
                    to: pubkey,
                    witnesses: Some(vec![witness0]),
                    ..PayCommand::default()
                }),
                signers: vec![read_keypair_file(&keypair_file).unwrap().into()],
            }
        );

        // Test Pay Subcommand w/ Timestamp
        let test_pay_timestamp = test_commands.clone().get_matches_from(vec![
            "test",
            "pay",
            &pubkey_string,
            "50",
            "--after",
            "2018-09-19T17:30:59",
            "--require-timestamp-from",
            &witness0_string,
        ]);
        assert_eq!(
            parse_command(&test_pay_timestamp, &keypair_file, &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::Pay(PayCommand {
                    amount: SpendAmount::Some(50_000_000_000),
                    to: pubkey,
                    timestamp: Some(dt),
                    timestamp_pubkey: Some(witness0),
                    ..PayCommand::default()
                }),
                signers: vec![read_keypair_file(&keypair_file).unwrap().into()],
            }
        );

        // Test Pay Subcommand w/ sign-only
        let blockhash = Hash::default();
        let blockhash_string = format!("{}", blockhash);
        let test_pay = test_commands.clone().get_matches_from(vec![
            "test",
            "pay",
            &pubkey_string,
            "50",
            "--blockhash",
            &blockhash_string,
            "--sign-only",
        ]);
        assert_eq!(
            parse_command(&test_pay, &keypair_file, &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::Pay(PayCommand {
                    amount: SpendAmount::Some(50_000_000_000),
                    to: pubkey,
                    blockhash_query: BlockhashQuery::None(blockhash),
                    sign_only: true,
                    ..PayCommand::default()
                }),
                signers: vec![read_keypair_file(&keypair_file).unwrap().into()],
            }
        );

        // Test Pay Subcommand w/ Blockhash
        let test_pay = test_commands.clone().get_matches_from(vec![
            "test",
            "pay",
            &pubkey_string,
            "50",
            "--blockhash",
            &blockhash_string,
        ]);
        assert_eq!(
            parse_command(&test_pay, &keypair_file, &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::Pay(PayCommand {
                    amount: SpendAmount::Some(50_000_000_000),
                    to: pubkey,
                    blockhash_query: BlockhashQuery::FeeCalculator(
                        blockhash_query::Source::Cluster,
                        blockhash
                    ),
                    ..PayCommand::default()
                }),
                signers: vec![read_keypair_file(&keypair_file).unwrap().into()],
            }
        );

        // Test Pay Subcommand w/ Nonce
        let blockhash = Hash::default();
        let blockhash_string = format!("{}", blockhash);
        let test_pay = test_commands.clone().get_matches_from(vec![
            "test",
            "pay",
            &pubkey_string,
            "50",
            "--blockhash",
            &blockhash_string,
            "--nonce",
            &pubkey_string,
        ]);
        assert_eq!(
            parse_command(&test_pay, &keypair_file, &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::Pay(PayCommand {
                    amount: SpendAmount::Some(50_000_000_000),
                    to: pubkey,
                    blockhash_query: BlockhashQuery::FeeCalculator(
                        blockhash_query::Source::NonceAccount(pubkey),
                        blockhash
                    ),
                    nonce_account: Some(pubkey),
                    ..PayCommand::default()
                }),
                signers: vec![read_keypair_file(&keypair_file).unwrap().into()],
            }
        );

        // Test Pay Subcommand w/ Nonce and Nonce Authority
        let blockhash = Hash::default();
        let blockhash_string = format!("{}", blockhash);
        let keypair = read_keypair_file(&keypair_file).unwrap();
        let test_pay = test_commands.clone().get_matches_from(vec![
            "test",
            "pay",
            &pubkey_string,
            "50",
            "--blockhash",
            &blockhash_string,
            "--nonce",
            &pubkey_string,
            "--nonce-authority",
            &keypair_file,
        ]);
        assert_eq!(
            parse_command(&test_pay, &keypair_file, &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::Pay(PayCommand {
                    amount: SpendAmount::Some(50_000_000_000),
                    to: pubkey,
                    blockhash_query: BlockhashQuery::FeeCalculator(
                        blockhash_query::Source::NonceAccount(pubkey),
                        blockhash
                    ),
                    nonce_account: Some(pubkey),
                    nonce_authority: 0,
                    ..PayCommand::default()
                }),
                signers: vec![keypair.into()],
            }
        );

        // Test Pay Subcommand w/ Nonce and Offline Nonce Authority
        let keypair = read_keypair_file(&keypair_file).unwrap();
        let authority_pubkey = keypair.pubkey();
        let authority_pubkey_string = format!("{}", authority_pubkey);
        let sig = keypair.sign_message(&[0u8]);
        let signer_arg = format!("{}={}", authority_pubkey, sig);
        let test_pay = test_commands.clone().get_matches_from(vec![
            "test",
            "pay",
            &pubkey_string,
            "50",
            "--blockhash",
            &blockhash_string,
            "--nonce",
            &pubkey_string,
            "--nonce-authority",
            &authority_pubkey_string,
            "--signer",
            &signer_arg,
        ]);
        assert_eq!(
            parse_command(&test_pay, &keypair_file, &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::Pay(PayCommand {
                    amount: SpendAmount::Some(50_000_000_000),
                    to: pubkey,
                    blockhash_query: BlockhashQuery::FeeCalculator(
                        blockhash_query::Source::NonceAccount(pubkey),
                        blockhash
                    ),
                    nonce_account: Some(pubkey),
                    nonce_authority: 0,
                    ..PayCommand::default()
                }),
                signers: vec![keypair.into()],
            }
        );

        // Test Pay Subcommand w/ Nonce and Offline Nonce Authority
        // authority pubkey not in signers fails
        let keypair = read_keypair_file(&keypair_file).unwrap();
        let authority_pubkey = keypair.pubkey();
        let authority_pubkey_string = format!("{}", authority_pubkey);
        let sig = keypair.sign_message(&[0u8]);
        let signer_arg = format!("{}={}", Pubkey::new_rand(), sig);
        let test_pay = test_commands.clone().get_matches_from(vec![
            "test",
            "pay",
            &pubkey_string,
            "50",
            "--blockhash",
            &blockhash_string,
            "--nonce",
            &pubkey_string,
            "--nonce-authority",
            &authority_pubkey_string,
            "--signer",
            &signer_arg,
        ]);
        assert!(parse_command(&test_pay, &keypair_file, &mut None).is_err());

        // Test Send-Signature Subcommand
        let test_send_signature = test_commands.clone().get_matches_from(vec![
            "test",
            "send-signature",
            &pubkey_string,
            &pubkey_string,
        ]);
        assert_eq!(
            parse_command(&test_send_signature, &keypair_file, &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::Witness(pubkey, pubkey),
                signers: vec![read_keypair_file(&keypair_file).unwrap().into()],
            }
        );
        let test_pay_multiple_witnesses = test_commands.clone().get_matches_from(vec![
            "test",
            "pay",
            &pubkey_string,
            "50",
            "--after",
            "2018-09-19T17:30:59",
            "--require-signature-from",
            &witness0_string,
            "--require-timestamp-from",
            &witness0_string,
            "--require-signature-from",
            &witness1_string,
        ]);
        assert_eq!(
            parse_command(&test_pay_multiple_witnesses, &keypair_file, &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::Pay(PayCommand {
                    amount: SpendAmount::Some(50_000_000_000),
                    to: pubkey,
                    timestamp: Some(dt),
                    timestamp_pubkey: Some(witness0),
                    witnesses: Some(vec![witness0, witness1]),
                    ..PayCommand::default()
                }),
                signers: vec![read_keypair_file(&keypair_file).unwrap().into()],
            }
        );

        // Test Send-Timestamp Subcommand
        let test_send_timestamp = test_commands.clone().get_matches_from(vec![
            "test",
            "send-timestamp",
            &pubkey_string,
            &pubkey_string,
            "--date",
            "2018-09-19T17:30:59",
        ]);
        assert_eq!(
            parse_command(&test_send_timestamp, &keypair_file, &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::TimeElapsed(pubkey, pubkey, dt),
                signers: vec![read_keypair_file(&keypair_file).unwrap().into()],
            }
        );
        let test_bad_timestamp = test_commands.clone().get_matches_from(vec![
            "test",
            "send-timestamp",
            &pubkey_string,
            &pubkey_string,
            "--date",
            "20180919T17:30:59",
        ]);
        assert!(parse_command(&test_bad_timestamp, &keypair_file, &mut None).is_err());
    }

    #[test]
    fn test_cli_process_command() {
        // Success cases
        let mut config = CliConfig::default();
        config.rpc_client = Some(RpcClient::new_mock("succeeds".to_string()));

        let keypair = Keypair::new();
        let pubkey = keypair.pubkey().to_string();
        config.signers = vec![&keypair];
        config.command = CliCommand::Address;
        assert_eq!(process_command(&config).unwrap(), pubkey);

        config.command = CliCommand::Balance {
            pubkey: None,
            use_lamports_unit: true,
            commitment_config: CommitmentConfig::default(),
        };
        assert_eq!(process_command(&config).unwrap(), "50 lamports");

        config.command = CliCommand::Balance {
            pubkey: None,
            use_lamports_unit: false,
            commitment_config: CommitmentConfig::default(),
        };
        assert_eq!(process_command(&config).unwrap(), "0.00000005 SOL");

        let process_id = Pubkey::new_rand();
        config.command = CliCommand::Cancel(process_id);
        assert!(process_command(&config).is_ok());

        let good_signature = Signature::new(&bs58::decode(SIGNATURE).into_vec().unwrap());
        config.command = CliCommand::Confirm(good_signature);
        assert_eq!(process_command(&config).unwrap(), "Confirmed");

        let bob_keypair = Keypair::new();
        let bob_pubkey = bob_keypair.pubkey();
        let identity_keypair = Keypair::new();
        config.command = CliCommand::CreateVoteAccount {
            seed: None,
            identity_account: 2,
            authorized_voter: Some(bob_pubkey),
            authorized_withdrawer: Some(bob_pubkey),
            commission: 0,
        };
        config.signers = vec![&keypair, &bob_keypair, &identity_keypair];
        let result = process_command(&config);
        assert!(result.is_ok());

        let new_authorized_pubkey = Pubkey::new_rand();
        config.signers = vec![&bob_keypair];
        config.command = CliCommand::VoteAuthorize {
            vote_account_pubkey: bob_pubkey,
            new_authorized_pubkey,
            vote_authorize: VoteAuthorize::Voter,
        };
        let result = process_command(&config);
        assert!(result.is_ok());

        let new_identity_keypair = Keypair::new();
        config.signers = vec![&keypair, &bob_keypair, &new_identity_keypair];
        config.command = CliCommand::VoteUpdateValidator {
            vote_account_pubkey: bob_pubkey,
            new_identity_account: 2,
        };
        let result = process_command(&config);
        assert!(result.is_ok());

        let bob_keypair = Keypair::new();
        let bob_pubkey = bob_keypair.pubkey();
        let custodian = Pubkey::new_rand();
        config.command = CliCommand::CreateStakeAccount {
            stake_account: 1,
            seed: None,
            staker: None,
            withdrawer: None,
            lockup: Lockup {
                epoch: 0,
                unix_timestamp: 0,
                custodian,
            },
            amount: SpendAmount::Some(30),
            sign_only: false,
            blockhash_query: BlockhashQuery::All(blockhash_query::Source::Cluster),
            nonce_account: None,
            nonce_authority: 0,
            fee_payer: 0,
            from: 0,
        };
        config.signers = vec![&keypair, &bob_keypair];
        let result = process_command(&config);
        assert!(result.is_ok());

        let stake_pubkey = Pubkey::new_rand();
        let to_pubkey = Pubkey::new_rand();
        config.command = CliCommand::WithdrawStake {
            stake_account_pubkey: stake_pubkey,
            destination_account_pubkey: to_pubkey,
            lamports: 100,
            withdraw_authority: 0,
            custodian: None,
            sign_only: false,
            blockhash_query: BlockhashQuery::All(blockhash_query::Source::Cluster),
            nonce_account: None,
            nonce_authority: 0,
            fee_payer: 0,
        };
        config.signers = vec![&keypair];
        let result = process_command(&config);
        assert!(result.is_ok());

        let stake_pubkey = Pubkey::new_rand();
        config.command = CliCommand::DeactivateStake {
            stake_account_pubkey: stake_pubkey,
            stake_authority: 0,
            sign_only: false,
            blockhash_query: BlockhashQuery::default(),
            nonce_account: None,
            nonce_authority: 0,
            fee_payer: 0,
        };
        let result = process_command(&config);
        assert!(result.is_ok());

        let stake_pubkey = Pubkey::new_rand();
        let split_stake_account = Keypair::new();
        config.command = CliCommand::SplitStake {
            stake_account_pubkey: stake_pubkey,
            stake_authority: 0,
            sign_only: false,
            blockhash_query: BlockhashQuery::default(),
            nonce_account: None,
            nonce_authority: 0,
            split_stake_account: 1,
            seed: None,
            lamports: 30,
            fee_payer: 0,
        };
        config.signers = vec![&keypair, &split_stake_account];
        let result = process_command(&config);
        assert!(result.is_ok());

        config.command = CliCommand::GetSlot {
            commitment_config: CommitmentConfig::default(),
        };
        assert_eq!(process_command(&config).unwrap(), "0");

        config.command = CliCommand::GetTransactionCount {
            commitment_config: CommitmentConfig::default(),
        };
        assert_eq!(process_command(&config).unwrap(), "1234");

        config.signers = vec![&keypair];
        config.command = CliCommand::Pay(PayCommand {
            amount: SpendAmount::Some(10),
            to: bob_pubkey,
            ..PayCommand::default()
        });
        let result = process_command(&config);
        assert!(result.is_ok());

        let date_string = "\"2018-09-19T17:30:59Z\"";
        let dt: DateTime<Utc> = serde_json::from_str(&date_string).unwrap();
        config.command = CliCommand::Pay(PayCommand {
            amount: SpendAmount::Some(10),
            to: bob_pubkey,
            timestamp: Some(dt),
            timestamp_pubkey: Some(config.signers[0].pubkey()),
            ..PayCommand::default()
        });
        let result = process_command(&config);
        assert!(result.is_ok());

        let witness = Pubkey::new_rand();
        config.command = CliCommand::Pay(PayCommand {
            amount: SpendAmount::Some(10),
            to: bob_pubkey,
            witnesses: Some(vec![witness]),
            cancelable: true,
            ..PayCommand::default()
        });
        let result = process_command(&config);
        assert!(result.is_ok());

        let process_id = Pubkey::new_rand();
        config.command = CliCommand::TimeElapsed(bob_pubkey, process_id, dt);
        config.signers = vec![&keypair];
        let result = process_command(&config);
        assert!(result.is_ok());

        let witness = Pubkey::new_rand();
        config.command = CliCommand::Witness(bob_pubkey, witness);
        let result = process_command(&config);
        assert!(result.is_ok());

        // CreateAddressWithSeed
        let from_pubkey = Pubkey::new_rand();
        config.signers = vec![];
        config.command = CliCommand::CreateAddressWithSeed {
            from_pubkey: Some(from_pubkey),
            seed: "seed".to_string(),
            program_id: solana_stake_program::id(),
        };
        let address = process_command(&config);
        let expected_address =
            Pubkey::create_with_seed(&from_pubkey, "seed", &solana_stake_program::id()).unwrap();
        assert_eq!(address.unwrap(), expected_address.to_string());

        // Need airdrop cases
        let to = Pubkey::new_rand();
        config.signers = vec![&keypair];
        config.command = CliCommand::Airdrop {
            faucet_host: None,
            faucet_port: 1234,
            pubkey: Some(to),
            lamports: 50,
        };
        assert!(process_command(&config).is_ok());

        config.command = CliCommand::TimeElapsed(bob_pubkey, process_id, dt);
        let result = process_command(&config);
        assert!(result.is_ok());

        let witness = Pubkey::new_rand();
        config.command = CliCommand::Witness(bob_pubkey, witness);
        let result = process_command(&config);
        assert!(result.is_ok());

        // sig_not_found case
        config.rpc_client = Some(RpcClient::new_mock("sig_not_found".to_string()));
        let missing_signature = Signature::new(&bs58::decode("5VERv8NMvzbJMEkV8xnrLkEaWRtSz9CosKDYjCJjBRnbJLgp8uirBgmQpjKhoR4tjF3ZpRzrFmBV6UjKdiSZkQUW").into_vec().unwrap());
        config.command = CliCommand::Confirm(missing_signature);
        assert_eq!(process_command(&config).unwrap(), "Not found");

        // Tx error case
        config.rpc_client = Some(RpcClient::new_mock("account_in_use".to_string()));
        let any_signature = Signature::new(&bs58::decode(SIGNATURE).into_vec().unwrap());
        config.command = CliCommand::Confirm(any_signature);
        assert_eq!(
            process_command(&config).unwrap(),
            format!("Transaction failed: {}", TransactionError::AccountInUse)
        );

        // Failure cases
        config.rpc_client = Some(RpcClient::new_mock("fails".to_string()));

        config.command = CliCommand::Airdrop {
            faucet_host: None,
            faucet_port: 1234,
            pubkey: None,
            lamports: 50,
        };
        assert!(process_command(&config).is_err());

        config.command = CliCommand::Balance {
            pubkey: None,
            use_lamports_unit: false,
            commitment_config: CommitmentConfig::default(),
        };
        assert!(process_command(&config).is_err());

        let bob_keypair = Keypair::new();
        let identity_keypair = Keypair::new();
        config.command = CliCommand::CreateVoteAccount {
            seed: None,
            identity_account: 2,
            authorized_voter: Some(bob_pubkey),
            authorized_withdrawer: Some(bob_pubkey),
            commission: 0,
        };
        config.signers = vec![&keypair, &bob_keypair, &identity_keypair];
        assert!(process_command(&config).is_err());

        config.command = CliCommand::VoteAuthorize {
            vote_account_pubkey: bob_pubkey,
            new_authorized_pubkey: bob_pubkey,
            vote_authorize: VoteAuthorize::Voter,
        };
        assert!(process_command(&config).is_err());

        config.command = CliCommand::VoteUpdateValidator {
            vote_account_pubkey: bob_pubkey,
            new_identity_account: 1,
        };
        assert!(process_command(&config).is_err());

        config.command = CliCommand::GetSlot {
            commitment_config: CommitmentConfig::default(),
        };
        assert!(process_command(&config).is_err());

        config.command = CliCommand::GetTransactionCount {
            commitment_config: CommitmentConfig::default(),
        };
        assert!(process_command(&config).is_err());

        config.command = CliCommand::Pay(PayCommand {
            amount: SpendAmount::Some(10),
            to: bob_pubkey,
            ..PayCommand::default()
        });
        assert!(process_command(&config).is_err());

        config.command = CliCommand::Pay(PayCommand {
            amount: SpendAmount::Some(10),
            to: bob_pubkey,
            timestamp: Some(dt),
            timestamp_pubkey: Some(config.signers[0].pubkey()),
            ..PayCommand::default()
        });
        assert!(process_command(&config).is_err());

        config.command = CliCommand::Pay(PayCommand {
            amount: SpendAmount::Some(10),
            to: bob_pubkey,
            witnesses: Some(vec![witness]),
            cancelable: true,
            ..PayCommand::default()
        });
        assert!(process_command(&config).is_err());

        config.command = CliCommand::TimeElapsed(bob_pubkey, process_id, dt);
        assert!(process_command(&config).is_err());
    }

    #[test]
    fn test_cli_deploy() {
        solana_logger::setup();
        let mut pathbuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        pathbuf.push("tests");
        pathbuf.push("fixtures");
        pathbuf.push("noop");
        pathbuf.set_extension("so");

        // Success case
        let mut config = CliConfig::default();
        config.rpc_client = Some(RpcClient::new_mock("deploy_succeeds".to_string()));
        let default_keypair = Keypair::new();
        config.signers = vec![&default_keypair];

        config.command = CliCommand::Deploy(pathbuf.to_str().unwrap().to_string());
        let result = process_command(&config);
        let json: Value = serde_json::from_str(&result.unwrap()).unwrap();
        let program_id = json
            .as_object()
            .unwrap()
            .get("programId")
            .unwrap()
            .as_str()
            .unwrap();

        assert!(program_id.parse::<Pubkey>().is_ok());

        // Failure case
        config.command = CliCommand::Deploy("bad/file/location.so".to_string());
        assert!(process_command(&config).is_err());
    }

    #[test]
    fn test_parse_transfer_subcommand() {
        let test_commands = app("test", "desc", "version");

        let default_keypair = Keypair::new();
        let default_keypair_file = make_tmp_path("keypair_file");
        write_keypair_file(&default_keypair, &default_keypair_file).unwrap();

        //Test Transfer Subcommand, SOL
        let from_keypair = keypair_from_seed(&[0u8; 32]).unwrap();
        let from_pubkey = from_keypair.pubkey();
        let from_string = from_pubkey.to_string();
        let to_keypair = keypair_from_seed(&[1u8; 32]).unwrap();
        let to_pubkey = to_keypair.pubkey();
        let to_string = to_pubkey.to_string();
        let test_transfer = test_commands
            .clone()
            .get_matches_from(vec!["test", "transfer", &to_string, "42"]);
        assert_eq!(
            parse_command(&test_transfer, &default_keypair_file, &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::Transfer {
                    amount: SpendAmount::Some(42_000_000_000),
                    to: to_pubkey,
                    from: 0,
                    sign_only: false,
                    no_wait: false,
                    blockhash_query: BlockhashQuery::All(blockhash_query::Source::Cluster),
                    nonce_account: None,
                    nonce_authority: 0,
                    fee_payer: 0,
                },
                signers: vec![read_keypair_file(&default_keypair_file).unwrap().into()],
            }
        );

        // Test Transfer ALL
        let test_transfer = test_commands
            .clone()
            .get_matches_from(vec!["test", "transfer", &to_string, "ALL"]);
        assert_eq!(
            parse_command(&test_transfer, &default_keypair_file, &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::Transfer {
                    amount: SpendAmount::All,
                    to: to_pubkey,
                    from: 0,
                    sign_only: false,
                    no_wait: false,
                    blockhash_query: BlockhashQuery::All(blockhash_query::Source::Cluster),
                    nonce_account: None,
                    nonce_authority: 0,
                    fee_payer: 0,
                },
                signers: vec![read_keypair_file(&default_keypair_file).unwrap().into()],
            }
        );

        // Test Transfer no-wait
        let test_transfer = test_commands.clone().get_matches_from(vec![
            "test",
            "transfer",
            "--no-wait",
            &to_string,
            "42",
        ]);
        assert_eq!(
            parse_command(&test_transfer, &default_keypair_file, &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::Transfer {
                    amount: SpendAmount::Some(42_000_000_000),
                    to: to_pubkey,
                    from: 0,
                    sign_only: false,
                    no_wait: true,
                    blockhash_query: BlockhashQuery::All(blockhash_query::Source::Cluster),
                    nonce_account: None,
                    nonce_authority: 0,
                    fee_payer: 0,
                },
                signers: vec![read_keypair_file(&default_keypair_file).unwrap().into()],
            }
        );

        //Test Transfer Subcommand, offline sign
        let blockhash = Hash::new(&[1u8; 32]);
        let blockhash_string = blockhash.to_string();
        let test_transfer = test_commands.clone().get_matches_from(vec![
            "test",
            "transfer",
            &to_string,
            "42",
            "--blockhash",
            &blockhash_string,
            "--sign-only",
        ]);
        assert_eq!(
            parse_command(&test_transfer, &default_keypair_file, &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::Transfer {
                    amount: SpendAmount::Some(42_000_000_000),
                    to: to_pubkey,
                    from: 0,
                    sign_only: true,
                    no_wait: false,
                    blockhash_query: BlockhashQuery::None(blockhash),
                    nonce_account: None,
                    nonce_authority: 0,
                    fee_payer: 0,
                },
                signers: vec![read_keypair_file(&default_keypair_file).unwrap().into()],
            }
        );

        //Test Transfer Subcommand, submit offline `from`
        let from_sig = from_keypair.sign_message(&[0u8]);
        let from_signer = format!("{}={}", from_pubkey, from_sig);
        let test_transfer = test_commands.clone().get_matches_from(vec![
            "test",
            "transfer",
            &to_string,
            "42",
            "--from",
            &from_string,
            "--fee-payer",
            &from_string,
            "--signer",
            &from_signer,
            "--blockhash",
            &blockhash_string,
        ]);
        assert_eq!(
            parse_command(&test_transfer, &default_keypair_file, &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::Transfer {
                    amount: SpendAmount::Some(42_000_000_000),
                    to: to_pubkey,
                    from: 0,
                    sign_only: false,
                    no_wait: false,
                    blockhash_query: BlockhashQuery::FeeCalculator(
                        blockhash_query::Source::Cluster,
                        blockhash
                    ),
                    nonce_account: None,
                    nonce_authority: 0,
                    fee_payer: 0,
                },
                signers: vec![Presigner::new(&from_pubkey, &from_sig).into()],
            }
        );

        //Test Transfer Subcommand, with nonce
        let nonce_address = Pubkey::new(&[1u8; 32]);
        let nonce_address_string = nonce_address.to_string();
        let nonce_authority = keypair_from_seed(&[2u8; 32]).unwrap();
        let nonce_authority_file = make_tmp_path("nonce_authority_file");
        write_keypair_file(&nonce_authority, &nonce_authority_file).unwrap();
        let test_transfer = test_commands.clone().get_matches_from(vec![
            "test",
            "transfer",
            &to_string,
            "42",
            "--blockhash",
            &blockhash_string,
            "--nonce",
            &nonce_address_string,
            "--nonce-authority",
            &nonce_authority_file,
        ]);
        assert_eq!(
            parse_command(&test_transfer, &default_keypair_file, &mut None).unwrap(),
            CliCommandInfo {
                command: CliCommand::Transfer {
                    amount: SpendAmount::Some(42_000_000_000),
                    to: to_pubkey,
                    from: 0,
                    sign_only: false,
                    no_wait: false,
                    blockhash_query: BlockhashQuery::FeeCalculator(
                        blockhash_query::Source::NonceAccount(nonce_address),
                        blockhash
                    ),
                    nonce_account: Some(nonce_address.into()),
                    nonce_authority: 1,
                    fee_payer: 0,
                },
                signers: vec![
                    read_keypair_file(&default_keypair_file).unwrap().into(),
                    read_keypair_file(&nonce_authority_file).unwrap().into()
                ],
            }
        );
    }

    #[test]
    fn test_return_signers() {
        struct BadSigner {
            pubkey: Pubkey,
        }

        impl BadSigner {
            pub fn new(pubkey: Pubkey) -> Self {
                Self { pubkey }
            }
        }

        impl Signer for BadSigner {
            fn try_pubkey(&self) -> Result<Pubkey, SignerError> {
                Ok(self.pubkey)
            }

            fn try_sign_message(&self, _message: &[u8]) -> Result<Signature, SignerError> {
                Ok(Signature::new(&[1u8; 64]))
            }
        }

        let mut config = CliConfig::default();
        config.output_format = OutputFormat::JsonCompact;
        let present: Box<dyn Signer> = Box::new(keypair_from_seed(&[2u8; 32]).unwrap());
        let absent: Box<dyn Signer> = Box::new(NullSigner::new(&Pubkey::new(&[3u8; 32])));
        let bad: Box<dyn Signer> = Box::new(BadSigner::new(Pubkey::new(&[4u8; 32])));
        let to = Pubkey::new(&[5u8; 32]);
        let nonce = Pubkey::new(&[6u8; 32]);
        let from = present.pubkey();
        let fee_payer = absent.pubkey();
        let nonce_auth = bad.pubkey();
        let mut tx = Transaction::new_unsigned(Message::new_with_nonce(
            vec![system_instruction::transfer(&from, &to, 42)],
            Some(&fee_payer),
            &nonce,
            &nonce_auth,
        ));

        let signers = vec![present.as_ref(), absent.as_ref(), bad.as_ref()];
        let blockhash = Hash::new(&[7u8; 32]);
        tx.try_partial_sign(&signers, blockhash).unwrap();
        let res = return_signers(&tx, &config).unwrap();
        let sign_only = parse_sign_only_reply_string(&res);
        assert_eq!(sign_only.blockhash, blockhash);
        assert_eq!(sign_only.present_signers[0].0, present.pubkey());
        assert_eq!(sign_only.absent_signers[0], absent.pubkey());
        assert_eq!(sign_only.bad_signers[0], bad.pubkey());
    }
}
