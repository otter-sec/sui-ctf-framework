use std::collections::BTreeMap;
use std::path::Path;
use std::io::Write;
use std::fs::File;
use std::assert;
use std::error;

use once_cell::sync::Lazy;
use tempfile::NamedTempFile;

use sui_transactional_test_runner::{
    args::{
        SuiInitArgs, 
        SuiPublishArgs, 
        SuiRunArgs, 
        SuiSubcommand, 
        SuiValue, 
        ViewObjectCommand, 
        ProgrammableTransactionCommand
    },
    test_adapter::{
        FakeID, 
        SuiTestAdapter
    }
};
use sui_protocol_config::{
    ProtocolConfig, 
    ProtocolVersion, 
    Chain
};
pub use sui_types::{
    object::Object, 
    MOVE_STDLIB_ADDRESS, 
    SUI_FRAMEWORK_ADDRESS
};
use move_symbol_pool::Symbol;
use move_binary_format::file_format::CompiledModule;
use move_bytecode_source_map::{source_map::SourceMap};
use move_command_line_common::{
    address::ParsedAddress,
    values::ParsedValue
};
pub use move_compiler::{
    diagnostics::report_diagnostics,
    shared::{NumberFormat, NumericalAddress, PackagePaths},
    Flags, FullyCompiledProgram, construct_pre_compiled_lib
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, TypeTag},
    u256::U256,
};
use move_transactional_test_runner::{
    framework::{MoveTestAdapter, MaybeNamedCompiledModule},
    tasks::{InitCommand, SyntaxChoice, TaskInput},
};

static NAMED_ADDRESSES: Lazy<BTreeMap<String, NumericalAddress>> = Lazy::new(|| {
    let mut map = move_stdlib::move_stdlib_named_addresses();
    assert!(map.get("std").unwrap().into_inner() == MOVE_STDLIB_ADDRESS);

    map.insert(
        "sui".to_string(),
        NumericalAddress::new(SUI_FRAMEWORK_ADDRESS.into_bytes(), NumberFormat::Hex),
    );
    map
});

pub fn get_precompiled(sui_files: &Path) -> FullyCompiledProgram {
    let sui_sources: String = {
        let mut buf = sui_files.to_path_buf();
        buf.push("Sui");
        buf.to_string_lossy().to_string()
    };

    let sui_deps = {
        let mut buf = sui_files.to_path_buf();
        buf.push("MoveStdlib");
        buf.to_string_lossy().to_string()
    };

    let named_address_map = NAMED_ADDRESSES.clone();

    let fully_compiled_res = construct_pre_compiled_lib(
        vec![PackagePaths {
            name: None,
            paths: vec![sui_sources, sui_deps],
            named_address_map,
        }],
        None,
        Flags::empty(),
    )
    .unwrap();

    match fully_compiled_res {
        Err((files, diags)) => {
            eprintln!("[*] Sui framework failed to compile!");
            report_diagnostics(&files, diags)
        }
        Ok(res) => res,
    }
}

pub async fn initialize<'a>(
    named_addresses: Vec<(String, NumericalAddress)>,
    deps: &'a FullyCompiledProgram,
    accounts: Option<Vec<String>>,
) -> SuiTestAdapter<'a> {
    let protocol_version = Some(ProtocolConfig::get_for_version(ProtocolVersion::MAX, Chain::Unknown).version.as_u64());
    let command = (
        InitCommand { named_addresses }, 
        SuiInitArgs { 
            accounts: accounts, 
            protocol_version: protocol_version, 
            max_gas: None,
            shared_object_deletion: None,
            simulator: false, // true - test_adapter.rs line 309 & 319
            custom_validator_account: false,
            reference_gas_price: None, // Some(234)
            default_gas_price: None,   // Some(1000)
            object_snapshot_min_checkpoint_lag: None,
            object_snapshot_max_checkpoint_lag: None
        });
    let name = "init".to_string();
    let number = 0;
    let start_line = 1;
    let command_lines_stop = 1;
    let stop_line = 1;
    let data = None;

    let init_opt: Option<TaskInput<(InitCommand, SuiInitArgs)>> = Some(TaskInput {
        command,
        name,
        number,
        start_line,
        command_lines_stop,
        stop_line,
        data,
    });

    let default_syntax = SyntaxChoice::Source;
    let fully_compiled_program_opt = Some(deps);

    let (adapter, _result_opt) =
        SuiTestAdapter::init( default_syntax, fully_compiled_program_opt, init_opt, Path::new("") ).await;
    println!("[*] Initialization Result: {:#?}", _result_opt);
    println!("[*] Successfully Initialized");

    adapter
}

pub async fn publish_compiled_module(
    adapter: &mut SuiTestAdapter<'_>, 
    mod_bytes: Vec<u8>, 
    module_dependencies: Vec<String>, 
    sender: Option<String>
) -> AccountAddress {
    let mut modules : Vec<MaybeNamedCompiledModule> = Vec::new();
    let module : CompiledModule = CompiledModule::deserialize_with_defaults(&mod_bytes).unwrap();
    let named_addr_opt: Option<Symbol> = None;
    let source_map : Option<SourceMap> = None;
    
    let maybe_ncm = MaybeNamedCompiledModule {
        named_address: named_addr_opt,
        module: module,
        source_map: source_map,
    };
    
    modules.push( maybe_ncm );

    let gas_budget: Option<u64> = None;
    let extra: SuiPublishArgs = SuiPublishArgs { 
        sender: sender, 
        upgradeable: true, 
        dependencies: module_dependencies,
        gas_price: None
    };

    let (output, modules) = adapter
        .publish_modules(modules, gas_budget, extra)
        .await
        .unwrap();

    println!(
        "[*] Successfully published at {:#?}",
        modules.first().unwrap().module.address_identifiers[0]
    );
    println!("[*] Output: {:#?} \n", output.unwrap());

    modules.first().unwrap().module.address_identifiers[0]
}

pub async fn call_function(
    adapter: &mut SuiTestAdapter<'_>,
    mod_addr: AccountAddress,
    mod_name: &str,
    fun_name: &str,
    args: Vec<SuiValue>,
    signer: Option<String>,
) -> Result<(), Box<dyn error::Error>> {
    let module_id: ModuleId = ModuleId::new(mod_addr, Identifier::new(mod_name).unwrap());
    let function: &IdentStr = IdentStr::new(fun_name).unwrap();
    let type_args: Vec<TypeTag> = Vec::new();
    let signers: Vec<ParsedAddress> = Vec::new();

    let gas_budget: Option<u64> = None;
    let extra_args: SuiRunArgs = SuiRunArgs {
        sender: signer,
        gas_price: None,
        summarize: false,
    };

    let (output, _return_values) = adapter.call_function(
        &module_id, function, type_args, signers, args, gas_budget, extra_args,
    ).await.unwrap();

    println!("[*] Successfully called {:#?}", fun_name);
    println!("[*] Output Call: {:#?}", output.unwrap());

    Ok(())
}

pub async fn view_object(
    adapter: &mut SuiTestAdapter<'_>, 
    id: FakeID
) {
    let arg_view = TaskInput {
        command: SuiSubcommand::ViewObject(ViewObjectCommand { id }),
        name: "blank".to_string(),
        number: 0,
        start_line: 1,
        command_lines_stop: 1,
        stop_line: 1,
        data: None,
    };

    let output = adapter.handle_subcommand(arg_view).await.unwrap();

    println!("[*] Successfully viewed object {:#?}", id);
    println!("[*] Output Call: {:#?}", output.unwrap());
}

pub async fn fund_account(
    adapter: &mut SuiTestAdapter<'_>, 
    sender: String, 
    amount: u64, 
    account_address: String
) {
    let mut input = Vec::new();
    input.push( ParsedValue::InferredNum(U256::from(amount)) );
    input.push( ParsedValue::Address(ParsedAddress::Named(account_address.clone())) );

    let temp_file = NamedTempFile::new().unwrap();
    let mut file = File::create(temp_file.path()).unwrap();
    let text_to_write = "\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n//> SplitCoins(Gas, [Input(0)]);\n//> TransferObjects([Result(0)], Input(1))";
    let _ = file.write_all(text_to_write.as_bytes());
    let _ = file.flush();

    let arg_view = TaskInput {
        command: SuiSubcommand::ProgrammableTransaction(ProgrammableTransactionCommand { 
            sender: Some(sender.to_string()), 
            gas_budget: Some(5000000000), 
            gas_price: Some(1000),
            dev_inspect: false,
            inputs: input,
        }),
        name: "blank".to_string(),
        number: 0,
        start_line: 1,
        command_lines_stop: 1,
        stop_line: 1,
        data: Some(temp_file),
    };

    let output = adapter.handle_subcommand(arg_view).await.unwrap();

    println!("[*] Successfully funded Address {:#?} with {:#?}", account_address, amount);
    println!("[*] Output Call: {:#?}", output.unwrap());
}