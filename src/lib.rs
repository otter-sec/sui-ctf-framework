use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::io::Write;
use std::sync::Arc;
use std::fs::File;
use std::error;

use once_cell::sync::Lazy;
use tempfile::NamedTempFile;
use serde_json::Value;

use sui_graphql_rpc::test_infra::cluster::SnapshotLagConfig;
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
        SuiTestAdapter,
        PRE_COMPILED
    }
};
pub use sui_types::{
    object::Object, 
    MOVE_STDLIB_ADDRESS, 
    SUI_FRAMEWORK_ADDRESS
};

use move_core_types::parsing::{
    address::ParsedAddress,
    values::ParsedValue,
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
    framework::{MoveTestAdapter, MaybeNamedCompiledModule, store_modules},
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

// Sui CTF framework environment
pub struct SuiTF {
    adapter: SuiTestAdapter,
    account_map: HashMap<AccountAddress, String>,
    package_map: HashMap<String, AccountAddress>,
}

impl SuiTF {
    fn _get_precompiled(
        sui_files: &Path
    ) -> Result<FullyCompiledProgram, Box<dyn std::error::Error>> {
        // Prepare paths for Sui framework and Move standard library sources
        let sui_sources = {
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

        // Compile the Sui framework and Move stdlib
        let fully_compiled_res_outer = construct_pre_compiled_lib(
            vec![PackagePaths {
                name: None,
                paths: vec![sui_sources, sui_deps],
                named_address_map,
            }],
            None,
            Flags::empty(),
            None,
        );

        // Handle any outer error
        let fully_compiled_res_inner = match fully_compiled_res_outer {
            Ok(inner) => inner,
            Err(err) => {
                let msg = format!("Failed to compile Move frameworks: {}", err);
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, msg)));
            }
        };

        // Handle compilation diagnostics
        match fully_compiled_res_inner {
            Err((_files, _diags)) => {
                eprintln!("[*] Sui framework failed to compile!");
                // report_diagnostics(&files, diags);
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other, 
                    "Failed to compile Sui Move framework"
                )))
            }
            Ok(res) => {
                Ok(res)
            }
        }
    }

    pub async fn initialize<'a>(
        named_addresses: Vec<(String, NumericalAddress)>,
        accounts: Option<Vec<String>>,
    ) -> Result<SuiTF, Box<dyn error::Error>> { 
        // Initialize the SuiTestAdapter with optional accounts and default protocol version
        // let protocol_version = Some(ProtocolConfig::get_for_version(ProtocolVersion::MAX, Chain::Unknown).version.as_u64());
        let protocol_version = None;
        let snapshot_config = SnapshotLagConfig {
            snapshot_min_lag: 5,
            sleep_duration: 0,
        };

        let command = (
            InitCommand { 
                named_addresses: named_addresses.clone() 
            }, 
            SuiInitArgs { 
                accounts,
                protocol_version,
                max_gas: None,
                shared_object_deletion: None,
                simulator: true, 
                custom_validator_account: false,
                reference_gas_price: None,
                default_gas_price: None, 
                snapshot_config: snapshot_config,
                flavor: None,
                epochs_to_keep: None,
                data_ingestion_path: None,
                rest_api_url: None,
            }
        );
        let name = "init".to_string();
        let number = 0;
        let start_line = 1;
        let command_lines_stop = 1;
        let stop_line = 1;
        let data = None;
        let command_text = "init --addresses challenge=0x0 solution=0x0";
        let task_text = "//#".to_owned() + &command_text.replace('\n', "\n//#");

        let init_opt: Option<TaskInput<(InitCommand, SuiInitArgs)>> = Some(TaskInput {
            command,
            name,
            number,
            start_line,
            command_lines_stop,
            stop_line,
            data,
            task_text,
        });

        let default_syntax = SyntaxChoice::Source;
        let fully_compiled_program_opt = Some(Arc::new(PRE_COMPILED.clone()));

        // Perform initialization (publishing frameworks and creating accounts)
        let (adapter, result_opt) = SuiTestAdapter::init(
            default_syntax, 
            fully_compiled_program_opt, 
            init_opt, 
            Path::new("")
        ).await;

        println!("[*] Initialization Result: {:#?}", result_opt);
        println!("[*] Successfully Initialized");

        let mut account_map = HashMap::new();
        for (name, num_addr) in named_addresses.iter() {
            let addr: AccountAddress = num_addr.into_inner();
            account_map.insert(addr, name.clone());
        }

        let sui_tf = SuiTF {
            adapter,
            account_map,
            package_map: HashMap::new(),
        };

        Ok(sui_tf)
    }

    pub async fn publish_compiled_module(
        &mut self, 
        modules: Vec<MaybeNamedCompiledModule>, 
        module_dependencies: Vec<String>, 
        sender: Option<String>
    ) -> Result<AccountAddress, Box<dyn error::Error>>  {
        if modules.is_empty() {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "No modules to publish")));
        }

        let gas_budget: Option<u64> = None;
        let extra = SuiPublishArgs { 
            sender,
            upgradeable: true, 
            dependencies: module_dependencies,
            gas_price: None
        };

        // Attempt to publish the compiled modules to the Sui environment
        let result = self.adapter.publish_modules(modules, gas_budget, extra).await;
        let (output, published_modules) = match result {
            Ok(res) => res,
            Err(e) => {
                eprintln!("[!] Failed to publish modules: {:?}", e);
                return Err(e.into());
            }
        };

        if published_modules.is_empty() {
            return Err("No modules were published".into());
        }

        // Store the published modules and retrieve the address
        let package_name = published_modules[0].named_address.clone().unwrap().as_str().to_string();
        let published_address = published_modules[0].module.address_identifiers[0];
        let default_syntax = SyntaxChoice::Source;
        let data = NamedTempFile::new().expect("Failed to create temp file for modules");
        store_modules(&mut self.adapter, default_syntax, data, published_modules);

        println!("[*] Successfully published package '{}' at {:?}", package_name, published_address);
        println!("[*] Publish output: {:#?}\n", output.unwrap_or_else(|| "<no output>".to_string()));
        
        self.package_map.insert(package_name, published_address);

        Ok(published_address)
    }

    pub async fn call_function(
        &mut self,
        mod_addr: AccountAddress,
        mod_name: &str,
        fun_name: &str,
        args: Vec<SuiValue>,
        type_args: Vec<TypeTag>,
        signer: Option<String>,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        // Prepare module and function identifiers
        let module_id = ModuleId::new(mod_addr, Identifier::new(mod_name).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?);
        let function: &IdentStr = IdentStr::new(fun_name).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        let signers: Vec<ParsedAddress> = Vec::new();
        let gas_budget: Option<u64> = None;
        let extra_args = SuiRunArgs {
            sender: signer,
            gas_price: None,
            summarize: false,
        };

        // Call the Move function via the test adapter
        match self.adapter.call_function(
            &module_id, function, type_args, signers, args, gas_budget, extra_args,
        ).await {
            Ok((output, _return_values)) => {
                println!("[*] Successfully called {}", fun_name);
                println!("[*] Call output: {:#?}", output.clone().unwrap_or_else(|| "<empty>".to_string()));
                Ok(output)
            }
            Err(err) => {
                eprintln!("[!] Failed to call function: {:?}", err);
                Err(err.into())
            }
        }
    }

    pub async fn view_object(
        &mut self, 
        id: FakeID
    ) -> Result<Option<serde_json::Value>, Box<dyn std::error::Error>> {
        // Construct the command to view an object by its ID
        let command_text = "run".to_string();
        let task_text = "//#".to_owned() + &command_text.replace('\n', "\n//#");
        let arg_view = TaskInput {
            command: SuiSubcommand::ViewObject(ViewObjectCommand { id }),
            name: "view-object".to_string(),
            number: 0,
            start_line: 1,
            command_lines_stop: 1,
            stop_line: 1,
            data: None,
            task_text,
        };

        // Execute the view command
        match self.adapter.handle_subcommand(arg_view).await {
            Ok(out) => {
                println!("[*] Successfully viewed object {:#?}", id);
                if let Some(output_str) = out {
                    let parsed_output = Self::parse_output(&output_str);
                    Ok(Some(parsed_output))
                } else {
                    Ok(None)
                }
            }
            Err(err) => {
                eprintln!("[!] Failed to view object: {:?}", err);
                Err(err.into())
            }
        }
    }
    fn parse_output(
        output: &str
    ) -> Value {
        let mut lines = output.lines();
        let mut result = serde_json::Map::new();

        while let Some(line) = lines.next() {
            if let Some((key, value)) = line.split_once(": ") {
                let key = key.trim().to_string();
                let value = value.trim();

                if value.ends_with('{') {
                    let nested_object = Self::parse_nested_object(&mut lines, value);
                    result.insert(key, nested_object);
                } else {
                    result.insert(key, Value::String(value.to_string()));
                }
            }
        }

        Value::Object(result)
    }

    fn parse_nested_object(
        lines: &mut std::str::Lines, 
        initial_value: &str
    ) -> Value {
        let mut nested_result = serde_json::Map::new();
        let mut buffer = String::new();

        if !initial_value.trim_end().ends_with('}') {
            buffer.push_str(initial_value);
        }

        while let Some(line) = lines.next() {
            buffer.push_str("\n");
            buffer.push_str(line.trim());

            if line.trim().ends_with('}') {
                break;
            }
        }

        let inner_content = buffer
            .trim_start_matches('{')
            .trim_end_matches('}')
            .trim();

        let mut inner_lines = inner_content.lines();

        while let Some(line) = inner_lines.next() {
            if let Some((key, value)) = line.split_once(": ") {
                let key = key.trim().to_string();
                let value = value.trim();

                if value.ends_with('{') {
                    let nested_object = Self::parse_nested_object(&mut inner_lines, value);
                    nested_result.insert(key, nested_object);
                } else {
                    nested_result.insert(key, Value::String(value.to_string()));
                }
            }
        }

        Value::Object(nested_result)
    }

    pub async fn fund_account(
        &mut self,
        account_address: String,
        amount: u64,
        sender: AccountAddress
    ) -> Result<(), Box<dyn error::Error>> {
        // Prepare inputs for a programmable transaction to fund an address
        let mut input = vec![];
        input.push(ParsedValue::InferredNum(U256::from(amount)));
        input.push(ParsedValue::Address(ParsedAddress::Named(account_address.to_string())));

        // Create a temporary Move script that splits and transfers coins
        let temp_file = NamedTempFile::new().map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        {
            let mut file = File::create(temp_file.path()).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            let txn_script = "\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n//> SplitCoins(Gas, [Input(0)]);\n//> TransferObjects([Result(0)], Input(1))";
            file.write_all(txn_script.as_bytes()).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            file.flush().map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        }

        let command_text = "run".to_string();
        let task_text = "//#".to_owned() + &command_text.replace('\n', "\n//#");
        let arg_view = TaskInput {
            command: SuiSubcommand::ProgrammableTransaction(ProgrammableTransactionCommand { 
                sender: Some(sender.to_string()), 
                sponsor: None,
                gas_budget: Some(5_000_000_000), 
                gas_price: Some(1000),
                gas_payment: None,
                dev_inspect: false,
                dry_run: false,
                inputs: input,
            }),
            name: "fund".to_string(),
            number: 0,
            start_line: 1,
            command_lines_stop: 1,
            stop_line: 1,
            data: Some(temp_file),
            task_text: task_text,
        };

        // Execute the funding transaction
        match self.adapter.handle_subcommand(arg_view).await {
            Ok(out) => {
                println!("[*] Successfully funded account '{}' with {}", account_address, amount);
                println!("[*] Fund transaction output: {:#?}", out.unwrap_or_else(|| "<no output>".to_string()));
                Ok(())
            }
            Err(err) => {
                eprintln!("[!] Failed to fund address: {:?}", err);
                Err(err.into())
            }
        }
    }

    pub fn get_account_address(
        &self, 
        account_name: &str
    ) -> Option<AccountAddress> {
        self.account_map.iter().find_map(|(&addr, name)| {
            if name == account_name {
                Some(addr)
            } else {
                None
            }
        })
    }

    pub fn get_package_address(
        &self, 
        package_name: &str
    ) -> Option<AccountAddress> {
        self.package_map.get(package_name).cloned()
    }
}