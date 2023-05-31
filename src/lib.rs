use once_cell::sync::Lazy;
use std::assert;
use std::collections::BTreeMap;
use std::error;
use std::path::Path;

use sui_transactional_test_runner::args::{
    SuiInitArgs, SuiPublishArgs, SuiRunArgs, SuiSubcommand, SuiValue, ViewObjectCommand,
};
use sui_transactional_test_runner::test_adapter::{FakeID, SuiTestAdapter};
use sui_protocol_config::ProtocolConfig;
pub use sui_types;
pub use sui_types::{object::Object, MOVE_STDLIB_ADDRESS, SUI_FRAMEWORK_ADDRESS};

use move_symbol_pool::Symbol;
use move_binary_format::file_format::CompiledModule;
use move_command_line_common::address::ParsedAddress;
pub use move_compiler;
pub use move_compiler::{
    shared::{NumberFormat, NumericalAddress, PackagePaths},
    Flags, FullyCompiledProgram,
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, TypeTag},
};
pub use move_transactional_test_runner;
use move_transactional_test_runner::{
    framework::MoveTestAdapter,
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

    let fully_compiled_res = move_compiler::construct_pre_compiled_lib(
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
            move_compiler::diagnostics::report_diagnostics(&files, diags)
        }
        Ok(res) => res,
    }
}

pub fn initialize<'a>(
    named_addresses: Vec<(String, NumericalAddress)>,
    deps: &'a FullyCompiledProgram,
    accounts: Option<Vec<String>>,
) -> SuiTestAdapter<'a> {
    let protocol_version = Some(ProtocolConfig::get_for_max_version().version.as_u64());
    let command = (InitCommand { named_addresses }, SuiInitArgs { accounts, protocol_version });
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
        SuiTestAdapter::init(default_syntax, fully_compiled_program_opt, init_opt);
    println!("[*] Successfully Initialized");

    adapter
}

pub fn publish_compiled_module(adapter: &mut SuiTestAdapter, mod_bytes: Vec<u8>, module_dependencies: Vec<String>, sender: Option<String>) -> AccountAddress {
    let mut module : Vec<(Option<Symbol>, CompiledModule)> = Vec::new();
    let named_addr_opt: Option<Symbol> = None;
    module.push( (named_addr_opt, CompiledModule::deserialize_with_defaults(&mod_bytes).unwrap()) );

    let gas_budget: Option<u64> = None;
    let extra: SuiPublishArgs = SuiPublishArgs { sender: sender, upgradeable: true, dependencies: module_dependencies};

    let (output, module) = adapter
        .publish_modules(module, gas_budget, extra)
        .unwrap();

    println!(
        "[*] Successfully published at {:#?}",
        module.first().unwrap().1.address_identifiers[0]
    );
    println!("[*] Output: {:#?} \n", output.unwrap());

    module.first().unwrap().1.address_identifiers[0]
}

pub fn call_function(
    adapter: &mut SuiTestAdapter,
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
        protocol_version: None,
        uncharged: true,
    };

    let (output, _return_values) = adapter.call_function(
        &module_id, function, type_args, signers, args, gas_budget, extra_args,
    )?;

    println!("[*] Successfully called {:#?}", fun_name);
    println!("[*] Output Call: {:#?} \n", output.unwrap());

    Ok(())
}

pub fn view_object(adapter: &mut SuiTestAdapter, id: FakeID) {
    let arg_view = TaskInput {
        command: SuiSubcommand::ViewObject(ViewObjectCommand { id }),
        name: "blank".to_string(),
        number: 0,
        start_line: 1,
        command_lines_stop: 1,
        stop_line: 1,
        data: None,
    };

    let output = adapter.handle_subcommand(arg_view).unwrap();

    println!("[*] Successfully viewed object {:#?}", id);
    println!("[*] Output Call: {:#?} \n", output.unwrap());
}
