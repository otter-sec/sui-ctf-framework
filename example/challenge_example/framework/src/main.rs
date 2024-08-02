use std::env;
use std::error::Error;
use std::fmt;
use std::io::{Read, Write};
use std::mem::drop;
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::str::FromStr;

use serde_json::Value;

use tokio;

use move_transactional_test_runner::framework::{MaybeNamedCompiledModule};
use move_bytecode_source_map::{source_map::SourceMap, utils::source_map_from_file};
use move_binary_format::file_format::CompiledModule;
use move_symbol_pool::Symbol;
use move_core_types::{
    ident_str, 
    account_address::AccountAddress, 
    language_storage::{TypeTag, StructTag}};

use sui_types::Identifier;
use sui_ctf_framework::NumericalAddress;
use sui_transactional_test_runner::{args::SuiValue, test_adapter::FakeID};

async fn handle_client(mut stream: TcpStream) -> Result<(), Box<dyn Error>> {
    
    // Initialize SuiTestAdapter
    let modules = vec!["router", "OtterLoan", "OtterSwap", "ctf", "osec", "merch_store"];
    let sources = vec!["router", "otterloan", "otterswap", "ctf", "osec", "merchstore"];
    let mut deployed_modules: Vec<AccountAddress> = Vec::new();

    let named_addresses = vec![
        (
            "challenge".to_string(),
            NumericalAddress::parse_str(
                "0x0", 
            )?,
        ),
        (
            "solution".to_string(),
            NumericalAddress::parse_str(
                "0x0",
            )?,
        ),
        (
            "admin".to_string(),
            NumericalAddress::parse_str(
                "0xfccc9a421bbb13c1a66a1aa98f0ad75029ede94857779c6915b44f94068b921e",
            )?,
        ),
    ];

    let mut adapter = sui_ctf_framework::initialize(
        named_addresses,
        Some(vec!["challenger".to_string(), "solver".to_string()]),
    ).await;

    // Check Admin Account
    let object_output1 : Value = sui_ctf_framework::view_object(&mut adapter, FakeID::Enumerated(0, 0)).await;
    println!("Object Output: {:#?}", object_output1);
    
    let bytes_str = object_output1.get("Contents")
    .and_then(|contents| contents.get("id"))
    .and_then(|id| id.get("id"))
    .and_then(|inner_id| inner_id.get("bytes"))
    .and_then(|bytes| bytes.as_str())
    .unwrap();

    println!("Objet Bytes: {}", bytes_str);

    let mut mncp_modules : Vec<MaybeNamedCompiledModule> = Vec::new();

    for i in 0..modules.len() {

        let module = &modules[i];
        let _source = &sources[i];

        let mod_path = format!("./chall/build/challenge/bytecode_modules/{}.mv", module);
        let src_path = format!("./chall/build/challenge/source_maps/{}.mvsm", module);
        let mod_bytes: Vec<u8> = std::fs::read(mod_path)?;

        let module : CompiledModule = CompiledModule::deserialize_with_defaults(&mod_bytes).unwrap();
        let named_addr_opt: Option<Symbol> = Some(Symbol::from("challenge"));
        let source_map : Option<SourceMap> = Some(source_map_from_file(Path::new(&src_path)).unwrap());
        
        let maybe_ncm = MaybeNamedCompiledModule {
            named_address: named_addr_opt,
            module: module,
            source_map: source_map,
        };
        
        mncp_modules.push( maybe_ncm );
          
    }

    // Publish Challenge Module
    let mut chall_dependencies: Vec<String> = Vec::new();
    let chall_addr = sui_ctf_framework::publish_compiled_module(
        &mut adapter,
        mncp_modules,
        chall_dependencies,
        Some(String::from("challenger")),
    ).await;
    deployed_modules.push(chall_addr);
    println!("[SERVER] Module published at: {:?}", chall_addr); 

    let mut solution_data = [0 as u8; 2000];
    let _solution_size = stream.read(&mut solution_data)?;

    // Send Challenge Address
    let mut output = String::new();
    fmt::write(
        &mut output,
        format_args!(
            "[SERVER] Challenge modules published at: {}",
            chall_addr.to_string().as_str(),
        ),
    )
    .unwrap();
    stream.write(output.as_bytes()).unwrap();

    // Publish Solution Module
    let mut sol_dependencies: Vec<String> = Vec::new();
    sol_dependencies.push(String::from("challenge"));

    let mut mncp_solution : Vec<MaybeNamedCompiledModule> = Vec::new();
    let module : CompiledModule = CompiledModule::deserialize_with_defaults(&solution_data.to_vec()).unwrap();
    let named_addr_opt: Option<Symbol> = Some(Symbol::from("solution"));
    let source_map : Option<SourceMap> = None;
    
    let maybe_ncm = MaybeNamedCompiledModule {
        named_address: named_addr_opt,
        module: module,
        source_map: source_map,
    }; 
    mncp_solution.push( maybe_ncm );

    let sol_addr = sui_ctf_framework::publish_compiled_module(
        &mut adapter,
        mncp_solution,
        sol_dependencies,
        Some(String::from("solver")),
    ).await;
    println!("[SERVER] Solution published at: {:?}", sol_addr);

    // Send Solution Address
    output = String::new();
    fmt::write(
        &mut output,
        format_args!(
            "[SERVER] Solution published at {}",
            sol_addr.to_string().as_str()
        ),
    )
    .unwrap();
    stream.write(output.as_bytes()).unwrap();

    // Prepare Function Call Arguments
    let mut args_liq: Vec<SuiValue> = Vec::new();
    let arg_liq1 = SuiValue::Object(FakeID::Enumerated(2, 1), None); 
    let arg_liq2 = SuiValue::Object(FakeID::Enumerated(2, 5), None);
    let arg_liq3 = SuiValue::Object(FakeID::Enumerated(2, 6), None);
    args_liq.push(arg_liq1);
    args_liq.push(arg_liq2);
    args_liq.push(arg_liq3);

    let mut type_args : Vec<TypeTag> = Vec::new();
    let type1 = TypeTag::Struct(Box::new(StructTag {
        address: chall_addr,
        module: Identifier::from_str("ctf").unwrap(),
        name: Identifier::from_str("CTF").unwrap(),
        type_params: Vec::new(),
    }));
    let type2 = TypeTag::Struct(Box::new(StructTag {
        address: chall_addr,
        module: Identifier::from_str("osec").unwrap(),
        name: Identifier::from_str("OSEC").unwrap(),
        type_params: Vec::new(),
    }));
    type_args.push(type1);
    type_args.push(type2); 

    // Call Add Liquidity Function
    let ret_val = sui_ctf_framework::call_function(
        &mut adapter,
        chall_addr,
        "OtterSwap",
        "initialize_pool",
        args_liq,
        type_args,
        Some("challenger".to_string()),
    ).await;
    println!("[SERVER] Return value {:#?}", ret_val);
    println!("");

    // Prepare Function Call Arguments
    let mut args_sol: Vec<SuiValue> = Vec::new();
    let arg_ob1 = SuiValue::Object(FakeID::Enumerated(2, 1), None); 
    let arg_ob2 = SuiValue::Object(FakeID::Enumerated(2, 2), None); 
    args_sol.push(arg_ob1);
    args_sol.push(arg_ob2);

    let mut type_args_sol : Vec<TypeTag> = Vec::new();
    let sol_type1 = TypeTag::Struct(Box::new(StructTag {
        address: chall_addr,
        module: Identifier::from_str("ctf").unwrap(),
        name: Identifier::from_str("CTF").unwrap(),
        type_params: Vec::new(),
    }));
    let sol_type2 = TypeTag::Struct(Box::new(StructTag {
        address: chall_addr,
        module: Identifier::from_str("osec").unwrap(),
        name: Identifier::from_str("OSEC").unwrap(),
        type_params: Vec::new(),
    }));
    type_args_sol.push(sol_type1);
    type_args_sol.push(sol_type2); 

    // Call solve Function
    let ret_val = sui_ctf_framework::call_function(
        &mut adapter,
        sol_addr,
        "gringotts_solution",
        "solve",
        args_sol,
        type_args_sol,
        Some("solver".to_string()),
    ).await;
    println!("[SERVER] Return value {:#?}", ret_val);
    println!("");

    // Check Solution
    let mut args2: Vec<SuiValue> = Vec::new();
    let arg_ob2 = SuiValue::Object(FakeID::Enumerated(5, 0), None);
    args2.push(arg_ob2);

    let mut type_args_valid : Vec<TypeTag> = Vec::new();

    let sol_ret = sui_ctf_framework::call_function(
        &mut adapter,
        chall_addr,
        "merch_store",
        "has_flag",
        args2,
        type_args_valid,
        Some("solver".to_string()),
    ).await;
    println!("[SERVER] Return value {:#?}", sol_ret);
    println!("");

    // Validate Solution
    match sol_ret {
        Ok(_) => {
            println!("[SERVER] Correct Solution!");
            println!("");
            if let Ok(flag) = env::var("FLAG") {
                let message = format!("[SERVER] Congrats, flag: {}", flag);
                stream.write(message.as_bytes()).unwrap();
            } else {
                stream.write("[SERVER] Flag not found, please contact admin".as_bytes()).unwrap();
            }
        }
        Err(_error) => {
            println!("[SERVER] Invalid Solution!");
            println!("");
            stream.write("[SERVER] Invalid Solution!".as_bytes()).unwrap();
        }
    };

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create Socket - Port 31337
    let listener = TcpListener::bind("0.0.0.0:31337")?;
    println!("[SERVER] Starting server at port 31337!");

    let local = tokio::task::LocalSet::new();

    // Wait For Incoming Solution
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("[SERVER] New connection: {}", stream.peer_addr()?);
                    let result = local.run_until( async move {
                        tokio::task::spawn_local( async {
                            handle_client(stream).await.unwrap();
                        }).await.unwrap();
                    }).await;
                    println!("[SERVER] Result: {:?}", result);
            }
            Err(e) => {
                println!("[SERVER] Error: {}", e);
            }
        }
    }

    // Close Socket Server
    drop(listener);
    Ok(())
}
