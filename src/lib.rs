use std::assert;
use std::error;
use std::path::Path;
use once_cell::sync::Lazy;
use std::collections::BTreeMap;

use sui_transactional_test_runner::test_adapter::SuiTestAdapter;
use sui_types::{MOVE_STDLIB_ADDRESS, SUI_FRAMEWORK_ADDRESS, object::Object};
use sui_transactional_test_runner::args::{
    SuiInitArgs, SuiPublishArgs, SuiValue, 
    SuiSubcommand, SuiRunArgs, ViewObjectCommand
};

use move_binary_format::file_format::CompiledModule;
use move_command_line_common::address::ParsedAddress;
use move_compiler::{
    shared::{NumericalAddress, PackagePaths, NumberFormat},
    Flags, FullyCompiledProgram,
};
use move_transactional_test_runner::{
    framework::MoveTestAdapter,
    tasks::{SyntaxChoice, TaskInput, InitCommand},
};
use move_core_types::{
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, TypeTag},
    value::MoveValue,
    account_address::AccountAddress,
};



pub const DEFAULT_FRAMEWORK_PATH: &str = "/home/sui/crates/sui-framework";



static NAMED_ADDRESSES: Lazy<BTreeMap<String, NumericalAddress>> = Lazy::new(|| {

    let mut map = move_stdlib::move_stdlib_named_addresses();
    assert!(map.get("std").unwrap().into_inner() == MOVE_STDLIB_ADDRESS);
    
    map.insert(
        "sui".to_string(),
        NumericalAddress::new(
            SUI_FRAMEWORK_ADDRESS.into_bytes(),
            NumberFormat::Hex,
        ),
    );
    map
});



pub static PRE_COMPILED: Lazy<FullyCompiledProgram> = Lazy::new(|| {
    
    let sui_files: &Path = Path::new(DEFAULT_FRAMEWORK_PATH);
    let sui_sources: String = {
        let mut buf = sui_files.to_path_buf();
        buf.push("sources");
        buf.to_string_lossy().to_string()
    };

    let sui_deps = {
        let mut buf = sui_files.to_path_buf();
        buf.push("deps");
        buf.push("move-stdlib");
        buf.push("sources");
        buf.to_string_lossy().to_string()
    };

    let fully_compiled_res = move_compiler::construct_pre_compiled_lib(
        vec![PackagePaths {
            name: None,
            paths: vec![sui_sources, sui_deps],
            named_address_map: NAMED_ADDRESSES.clone(),
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
});



pub fn initialize( na_name : Vec<&str>, na_addr : Vec<[u8; 20]>, l_acc : Vec<&str> ) -> SuiTestAdapter<'static> {

    assert!(na_name.len() == na_addr.len());

    let mut named_addresses : Vec<(String, NumericalAddress)> = Vec::new();

    for i in 0..na_name.len() {
        named_addresses.push( 
            (
                String::from(na_name[i]), 
                NumericalAddress::new( na_addr[i], NumberFormat::Hex ) 
            ) 
        );
    }

    let mut account_data : Vec<String> = Vec::new();

    for i in 0..l_acc.len() {
        account_data.push( String::from(l_acc[i]) );
    }

    let accounts: Option<Vec<String>> = Some(account_data);

    let command = (InitCommand { named_addresses }, SuiInitArgs { accounts });
    let name = "init".to_string();
    let number = 0;
    let start_line = 1;
    let command_lines_stop = 1;
    let stop_line = 1;
    let data = None;

    let init_opt : Option<TaskInput<(InitCommand,SuiInitArgs)>> = Some(TaskInput {
        command,
        name,
        number,
        start_line,
        command_lines_stop,
        stop_line,
        data,
    });

    let default_syntax = SyntaxChoice::Source;
    let fully_compiled_program_opt = Some(&*PRE_COMPILED);
    
    let (mut adapter, result_opt) = SuiTestAdapter::init(default_syntax, fully_compiled_program_opt, init_opt);
    println!("[*] Successfully Initialized");
    println!("[*] Output: {:#?} \n", result_opt.unwrap());

    adapter
}



pub fn publish_compiled_module( adapter : &mut SuiTestAdapter, mod_bytes : Vec<u8> ) -> AccountAddress {

    let module = CompiledModule::deserialize(&mod_bytes).unwrap();

    let named_addr_opt: Option<Identifier> = None;
    let gas_budget: Option<u64> = None;
    let extra: SuiPublishArgs = SuiPublishArgs { sender: None };

    let (output, module) = adapter.publish_module(module, 
                                                  named_addr_opt, 
                                                  gas_budget, 
                                                  extra).unwrap();
                         
    println!("[*] Successfully published at {:#?}", module.address_identifiers[0]);
    println!("[*] Output: {:#?} \n", output.unwrap());

    module.address_identifiers[0]
}



pub fn call_function( adapter : &mut SuiTestAdapter, 
                      mod_addr : AccountAddress, 
                      mod_name : &str, 
                      fun_name : &str, 
                      args : Vec<SuiValue>, 
                      signer : Option<String>) -> Result<(), Box<dyn error::Error>> {
    
    let module_id : ModuleId = ModuleId::new(mod_addr, Identifier::new(mod_name).unwrap());
    let function: &IdentStr = IdentStr::new(fun_name).unwrap();
    let type_args: Vec<TypeTag> = Vec::new();
    let signers: Vec<ParsedAddress> = Vec::new();

    let gas_budget: Option<u64> = None;
    let extra_args : SuiRunArgs = SuiRunArgs{sender: signer, view_events: false};

    let (output, return_values) = adapter.call_function(
        &module_id,
        function,
        type_args,
        signers,
        args,
        gas_budget,
        extra_args,
    )?;

    println!("[*] Successfully called {:#?}", fun_name);
    println!("[*] Output Call: {:#?} \n", output.unwrap());

    Ok(())
}



pub fn view_object( adapter : &mut SuiTestAdapter, obj_id : u64 ) {

    let vcmd : ViewObjectCommand = ViewObjectCommand{
        id: obj_id,
    };

    let command : SuiSubcommand = SuiSubcommand::ViewObject(vcmd);
    let name = "blank".to_string();
    let number = 0;
    let start_line = 1;
    let command_lines_stop = 1;
    let stop_line = 1;
    let data = None;

    let arg_view = TaskInput {
        command,
        name,
        number,
        start_line,
        command_lines_stop,
        stop_line,
        data,
    };

    let output = adapter.handle_subcommand(arg_view).unwrap();
    
    println!("[*] Successfully viewed object {:#?}", obj_id);
    println!("[*] Output Call: {:#?} \n", output.unwrap());

    // This block of code and function signature can be used to return arbitrary Move Objects
    // but it requires a minor patch to the sui-framework:

    /* PATCH
        --- a/crates/sui-transactional-test-runner/src/test_adapter.rs
        +++ b/crates/sui-transactional-test-runner/src/test_adapter.rs
        @@ -65,7 +65,7 @@ const RNG_SEED: [u8; 32] = [
        
        pub struct SuiTestAdapter<'a> {
            vm: Arc<MoveVM>,
        -    pub(crate) storage: Arc<InMemoryStorage>,
        +    pub storage: Arc<InMemoryStorage>,
            native_functions: NativeFunctionTable,
            pub(crate) compiled_state: CompiledState<'a>,

        @@ -577,7 +578,7 @@ impl<'a> SuiTestAdapter<'a> {
            }
        }
    
        -    pub(crate) fn fake_to_real_object_id(&self, fake_id: FakeID) -> Option<ObjectID> {
        +    pub fn fake_to_real_object_id(&self, fake_id: FakeID) -> Option<ObjectID> {
                self.object_enumeration.get_by_right(&fake_id).copied()
            }
    */

    /* FUNCTION SIGNATURE
        pub fn view_object<'a>( adapter : &'a mut SuiTestAdapter, obj_id : u64 ) -> Option<&'a Object> {
    */

    /* CODE:
        let real_id = adapter.fake_to_real_object_id(obj_id);
        if let None = real_id {
            None
        }
        else {
            adapter.storage.get_object(&real_id.unwrap())
        }
    */
}
