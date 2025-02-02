use crate::starknet_commands::account::Account;
use crate::starknet_commands::show_config::ShowConfig;
use crate::starknet_commands::{
    account, call::Call, declare::Declare, deploy::Deploy, invoke::Invoke, multicall::Multicall,
    script::Script,
};
use anyhow::{anyhow, Result};

use camino::Utf8PathBuf;
use cast::helpers::constants::{DEFAULT_ACCOUNTS_FILE, DEFAULT_MULTICALL_CONTENTS};
use cast::helpers::scarb_utils::{parse_scarb_config, CastConfig};
use cast::{
    chain_id_to_network_name, get_account, get_block_id, get_chain_id, get_provider,
    print_command_result, ValueFormat,
};
use clap::{Parser, Subcommand};
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::JsonRpcClient;
use tokio::runtime::Runtime;

mod starknet_commands;

#[derive(Parser)]
#[command(version)]
#[command(about = "Cast - a Starknet Foundry CLI", long_about = None)]
#[clap(name = "sncast")]
#[allow(clippy::struct_excessive_bools)]
struct Cli {
    /// Profile name in Scarb.toml config file
    #[clap(short, long)]
    profile: Option<String>,

    /// Path to Scarb.toml that is to be used; overrides default behaviour of searching for scarb.toml in current or parent directories
    #[clap(short = 's', long)]
    path_to_scarb_toml: Option<Utf8PathBuf>,

    /// RPC provider url address; overrides url from Scarb.toml
    #[clap(short = 'u', long = "url")]
    rpc_url: Option<String>,

    /// Account to be used for contract declaration;
    /// When using keystore (`--keystore`), this should be a path to account file    
    /// When using accounts file, this should be an account name
    #[clap(short = 'a', long)]
    account: Option<String>,

    /// Path to the file holding accounts info
    #[clap(short = 'f', long = "accounts-file")]
    accounts_file_path: Option<Utf8PathBuf>,

    /// Path to keystore file; if specified, --account should be a path to starkli JSON account file
    #[clap(short, long)]
    keystore: Option<Utf8PathBuf>,

    /// If passed, values will be displayed as integers
    #[clap(long, conflicts_with = "hex_format")]
    int_format: bool,

    /// If passed, values will be displayed as hex
    #[clap(long, conflicts_with = "int_format")]
    hex_format: bool,

    /// If passed, output will be displayed in json format
    #[clap(short, long)]
    json: bool,

    /// If passed, command will wait until transaction is accepted or rejected
    #[clap(short, long)]
    wait: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Declare a contract
    Declare(Declare),

    /// Deploy a contract
    Deploy(Deploy),

    /// Call a contract
    Call(Call),

    /// Invoke a contract
    Invoke(Invoke),

    /// Execute multiple calls
    Multicall(Multicall),

    /// Create and deploy an account
    Account(Account),

    /// Show current configuration being used
    ShowConfig(ShowConfig),

    /// Run a deployment script
    Script(Script),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Clap validates that both are not passed at same time
    let value_format = if cli.hex_format {
        ValueFormat::Hex
    } else if cli.int_format {
        ValueFormat::Int
    } else {
        ValueFormat::Default
    };

    let mut config = parse_scarb_config(&cli.profile, &cli.path_to_scarb_toml)?;
    update_cast_config(&mut config, &cli);

    let provider = get_provider(&config.rpc_url)?;
    let runtime = Runtime::new().expect("Could not instantiate Runtime");

    if let Commands::Script(script) = cli.command {
        let mut result = starknet_commands::script::run(
            &script.script_module_name,
            &cli.path_to_scarb_toml,
            &provider,
            runtime,
            &config,
        );

        print_command_result("script", &mut result, value_format, cli.json)?;
        Ok(())
    } else {
        runtime.block_on(run_async_command(cli, config, provider, value_format))
    }
}

#[allow(clippy::too_many_lines)]
async fn run_async_command(
    cli: Cli,
    mut config: CastConfig,
    provider: JsonRpcClient<HttpTransport>,
    value_format: ValueFormat,
) -> Result<()> {
    match cli.command {
        Commands::Declare(declare) => {
            let account = get_account(
                &config.account,
                &config.accounts_file,
                &provider,
                &config.keystore,
            )
            .await?;
            let mut result = starknet_commands::declare::declare(
                &declare.contract,
                declare.max_fee,
                &account,
                &cli.path_to_scarb_toml,
                cli.wait,
            )
            .await;

            print_command_result("declare", &mut result, value_format, cli.json)?;
            Ok(())
        }
        Commands::Deploy(deploy) => {
            let account = get_account(
                &config.account,
                &config.accounts_file,
                &provider,
                &config.keystore,
            )
            .await?;
            let mut result = starknet_commands::deploy::deploy(
                deploy.class_hash,
                deploy.constructor_calldata,
                deploy.salt,
                deploy.unique,
                deploy.max_fee,
                &account,
                cli.wait,
            )
            .await;

            print_command_result("deploy", &mut result, value_format, cli.json)?;
            Ok(())
        }
        Commands::Call(call) => {
            let block_id = get_block_id(&call.block_id)?;

            let mut result = starknet_commands::call::call(
                call.contract_address,
                call.function.as_ref(),
                call.calldata,
                &provider,
                block_id.as_ref(),
            )
            .await;

            print_command_result("call", &mut result, value_format, cli.json)?;
            Ok(())
        }
        Commands::Invoke(invoke) => {
            let account = get_account(
                &config.account,
                &config.accounts_file,
                &provider,
                &config.keystore,
            )
            .await?;
            let mut result = starknet_commands::invoke::invoke(
                invoke.contract_address,
                &invoke.function,
                invoke.calldata,
                invoke.max_fee,
                &account,
                cli.wait,
            )
            .await;

            print_command_result("invoke", &mut result, value_format, cli.json)?;
            Ok(())
        }
        Commands::Multicall(multicall) => {
            match &multicall.command {
                starknet_commands::multicall::Commands::New(new) => {
                    if let Some(output_path) = &new.output_path {
                        let mut result =
                            starknet_commands::multicall::new::new(output_path, new.overwrite);
                        print_command_result("multicall new", &mut result, value_format, cli.json)?;
                    } else {
                        println!("{DEFAULT_MULTICALL_CONTENTS}");
                    }
                }
                starknet_commands::multicall::Commands::Run(run) => {
                    let account = get_account(
                        &config.account,
                        &config.accounts_file,
                        &provider,
                        &config.keystore,
                    )
                    .await?;
                    let mut result = starknet_commands::multicall::run::run(
                        &run.path,
                        &account,
                        run.max_fee,
                        cli.wait,
                    )
                    .await;

                    print_command_result("multicall run", &mut result, value_format, cli.json)?;
                }
            }
            Ok(())
        }
        Commands::Account(account) => match account.command {
            account::Commands::Add(add) => {
                config.account = add.name.clone();
                let mut result = starknet_commands::account::add::add(
                    &config.rpc_url,
                    &config.account,
                    &config.accounts_file,
                    &cli.path_to_scarb_toml,
                    &provider,
                    &add,
                )
                .await;

                print_command_result("account add", &mut result, value_format, cli.json)?;
                Ok(())
            }
            account::Commands::Create(create) => {
                let chain_id = get_chain_id(&provider).await?;
                if config.keystore == Utf8PathBuf::default() {
                    config.account = create
                        .name
                        .ok_or_else(|| anyhow!("required argument --name not provided"))?;
                }
                let mut result = starknet_commands::account::create::create(
                    &config.rpc_url,
                    &config.account,
                    &config.accounts_file,
                    &config.keystore,
                    &provider,
                    cli.path_to_scarb_toml,
                    chain_id,
                    create.salt,
                    create.add_profile,
                    create.class_hash,
                )
                .await;

                print_command_result("account create", &mut result, value_format, cli.json)?;
                Ok(())
            }
            account::Commands::Deploy(deploy) => {
                let chain_id = get_chain_id(&provider).await?;
                let keystore_path =
                    Some(config.keystore.clone()).filter(|p| *p != Utf8PathBuf::default());
                let account_path = Some(Utf8PathBuf::from(config.account.clone()))
                    .filter(|p| *p != String::default());
                if config.keystore == Utf8PathBuf::default() {
                    config.account = deploy
                        .name
                        .ok_or_else(|| anyhow!("required argument --name not provided"))?;
                }
                let mut result = starknet_commands::account::deploy::deploy(
                    &provider,
                    config.accounts_file,
                    config.account,
                    chain_id,
                    deploy.max_fee,
                    cli.wait,
                    deploy.class_hash,
                    keystore_path,
                    account_path,
                )
                .await;

                print_command_result("account deploy", &mut result, value_format, cli.json)?;
                Ok(())
            }
            account::Commands::Delete(delete) => {
                config.account = delete
                    .name
                    .ok_or_else(|| anyhow!("required argument --name not provided"))?;
                let network_name = match delete.network {
                    Some(network) => network,
                    None => chain_id_to_network_name(get_chain_id(&provider).await?),
                };

                let mut result = starknet_commands::account::delete::delete(
                    &config.account,
                    &config.accounts_file,
                    &cli.path_to_scarb_toml,
                    delete.delete_profile,
                    &network_name,
                );

                print_command_result("account delete", &mut result, value_format, cli.json)?;
                Ok(())
            }
        },
        Commands::ShowConfig(_) => {
            let mut result = starknet_commands::show_config::show_config(
                &provider,
                config,
                cli.profile,
                cli.path_to_scarb_toml,
            )
            .await;
            print_command_result("show-config", &mut result, value_format, cli.json)?;
            Ok(())
        }
        Commands::Script(_) => unreachable!(),
    }
}

fn update_cast_config(config: &mut CastConfig, cli: &Cli) {
    macro_rules! clone_or_else {
        ($field:expr, $config_field:expr) => {
            $field.clone().unwrap_or_else(|| $config_field.clone())
        };
    }

    config.rpc_url = clone_or_else!(cli.rpc_url, config.rpc_url);
    config.account = clone_or_else!(cli.account, config.account);
    config.keystore = clone_or_else!(cli.keystore, config.keystore);

    if config.accounts_file == Utf8PathBuf::default() {
        config.accounts_file = Utf8PathBuf::from(DEFAULT_ACCOUNTS_FILE);
    }
    let new_accounts_file = clone_or_else!(cli.accounts_file_path, config.accounts_file);

    config.accounts_file = Utf8PathBuf::from(shellexpand::tilde(&new_accounts_file).to_string());
}
