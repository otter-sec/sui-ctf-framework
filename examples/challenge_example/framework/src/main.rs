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

use move_transactional_test_runner::framework::{MaybeNamedCompiledModule, MoveTestAdapter};
use move_bytecode_source_map::{source_map::SourceMap, utils::source_map_from_file};
use move_binary_format::file_format::CompiledModule;
use move_symbol_pool::Symbol;
use move_core_types::{
    account_address::AccountAddress, 
    language_storage::{TypeTag, StructTag}};

use sui_types::Identifier;
use sui_ctf_framework::{NumericalAddress, SuiTF};
use sui_transactional_test_runner::{args::SuiValue, test_adapter::FakeID};

macro_rules! handle_err {
    ($stream:expr, $msg:expr, $err:expr) => {{
        let full = format!("[SERVER ERROR] {}: {}", $msg, $err);
        eprintln!("{}", full);
        let _ = $stream.write_all(full.as_bytes());   // ignore write failures
        drop($stream);                                // close socket
        return Err::<(), Box<dyn std::error::Error>>(full.into());
    }};
}

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

    let mut suitf = match sui_ctf_framework::SuiTF::initialize(
        named_addresses,
        Some(vec!["challenger".to_string(), "solver".to_string()]),
    ).await {
        Ok(adapter) => adapter,
        Err(e) => handle_err!(stream, "SuiTF initialization failed", e),
    };

    // Check Admin Account
    let object_output1 : Value = match suitf.view_object(FakeID::Enumerated(0, 0)).await {
        Ok(output) => {
            println!("[SERVER] Object Output: {:#?}", output);
            output.unwrap()
        }
        Err(e) => handle_err!(stream, "Error viewing object 0:0", e),
    };
    
    let bytes_str = match object_output1.get("Contents")
        .and_then(|c| c.get("id"))
        .and_then(|id| id.get("id"))
        .and_then(|inner| inner.get("bytes"))
        .and_then(|bytes| bytes.as_str()) {
            Some(s) => s.to_owned(),
            None => handle_err!(stream, "Malformed JSON response for object bytes", "missing field"),
        };

    println!("Objet Bytes: {}", bytes_str);

    let mut mncp_modules : Vec<MaybeNamedCompiledModule> = Vec::new();

    for (module, source) in modules.iter().zip(sources.iter()) {
        let mod_path = format!("./chall/build/challenge/bytecode_modules/{}.mv", module);
        let src_path = format!("./chall/build/challenge/debug_info/{}.json", module);
        let mod_bytes: Vec<u8> = match std::fs::read(mod_path) {
            Ok(data) => data,
            Err(e) => handle_err!(stream, format!("Failed to read {}", module), e),
        };

        let module: CompiledModule = match CompiledModule::deserialize_with_defaults(&mod_bytes) {
            Ok(data) => data,
            Err(e) => {
                return Err(Box::new(e))
            }
        }; 
        let named_addr_opt: Option<Symbol> = Some(Symbol::from("challenge"));
        let source_map: Option<SourceMap> = match source_map_from_file(Path::new(&src_path)) {
            Ok(data) => Some(data),
            Err(e) => handle_err!(stream, format!("Deserialization failed for {}", module.name()), e),
        };
        
        mncp_modules.push( MaybeNamedCompiledModule {
            named_address: named_addr_opt,
            module: module,
            source_map: source_map,
        });
    }

    // Publish Challenge Module
    let chall_dependencies: Vec<String> = Vec::new();
    let chall_addr = match suitf.publish_compiled_module(
        mncp_modules,
        chall_dependencies,
        Some(String::from("challenger")),
    ).await {
        Ok(addr) => addr,
        Err(e) => handle_err!(stream, "Challenge module publish failed", e),
    };

    deployed_modules.push(chall_addr);
    println!("[SERVER] Module published at: {:?}", chall_addr); 

    let mut solution_data = [0 as u8; 2000];
    let _solution_size = match stream.read(&mut solution_data) {
        Ok(size) => {
            if size == 0 {
                handle_err!(stream, "No data read from stream", "size is zero");
            } else{
                size
            }
        }
        Err(e) => handle_err!(stream, "Failed to read solution data", e),
    };

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
    stream.write_all(output.as_bytes())?;

    // Publish Solution Module
    let mut sol_dependencies: Vec<String> = Vec::new();
    sol_dependencies.push(String::from("challenge"));

    let mut mncp_solution : Vec<MaybeNamedCompiledModule> = Vec::new();
    let module : CompiledModule = match CompiledModule::deserialize_with_defaults(&solution_data.to_vec()) {
        Ok(m) => m,
        Err(e) => handle_err!(stream, "Solution deserialization failed", e),
    };
    let named_addr_opt: Option<Symbol> = Some(Symbol::from("solution"));
    let source_map : Option<SourceMap> = None;
    
    mncp_solution.push( MaybeNamedCompiledModule {
        named_address: named_addr_opt,
        module: module,
        source_map: source_map,
    });

    let sol_addr = match suitf.publish_compiled_module(
        mncp_solution,
        sol_dependencies,
        Some(String::from("solver")),
    ).await {
        Ok(addr) => addr,
        Err(e) => handle_err!(stream, "Solution module publish failed", e),
    };
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
    stream.write_all(output.as_bytes())?;

    // Prepare Function Call Arguments
    let mut args_liq: Vec<SuiValue> = Vec::new();
    args_liq.push(SuiValue::Object(FakeID::Enumerated(2, 1), None));
    args_liq.push(SuiValue::Object(FakeID::Enumerated(2, 5), None));
    args_liq.push(SuiValue::Object(FakeID::Enumerated(2, 6), None));

    let mut type_args : Vec<TypeTag> = Vec::new();
    type_args.push(TypeTag::Struct(Box::new(StructTag {
        address: chall_addr,
        module: Identifier::from_str("ctf").unwrap(),
        name: Identifier::from_str("CTF").unwrap(),
        type_params: Vec::new(),
    })));
    type_args.push(TypeTag::Struct(Box::new(StructTag {
        address: chall_addr,
        module: Identifier::from_str("osec").unwrap(),
        name: Identifier::from_str("OSEC").unwrap(),
        type_params: Vec::new(),
    }))); 

    // Call Add Liquidity Function
    let ret_val = match suitf.call_function(
        chall_addr,
        "OtterSwap",
        "initialize_pool",
        args_liq,
        type_args,
        Some("challenger".to_string()),
    ).await {
        Ok(output) => output,
        Err(e) => handle_err!(stream, "Calling initialize_pool failed", e),
    };
    println!("[SERVER] Return value {:#?}", ret_val);
    println!("");

    // Prepare Function Call Arguments
    let mut args_sol: Vec<SuiValue> = Vec::new();
    args_sol.push(SuiValue::Object(FakeID::Enumerated(2, 1), None));
    args_sol.push(SuiValue::Object(FakeID::Enumerated(2, 2), None));

    let mut type_args_sol : Vec<TypeTag> = Vec::new();
    type_args_sol.push(TypeTag::Struct(Box::new(StructTag {
        address: chall_addr,
        module: Identifier::from_str("ctf").unwrap(),
        name: Identifier::from_str("CTF").unwrap(),
        type_params: Vec::new(),
    })));
    type_args_sol.push(TypeTag::Struct(Box::new(StructTag {
        address: chall_addr,
        module: Identifier::from_str("osec").unwrap(),
        name: Identifier::from_str("OSEC").unwrap(),
        type_params: Vec::new(),
    }))); 

    // Call solve Function
    let ret_val = match suitf.call_function(
        sol_addr,
        "gringotts_solution",
        "solve",
        args_sol,
        type_args_sol,
        Some("solver".to_string()),
    ).await {
        Ok(output) => output,
        Err(e) => handle_err!(stream, "Calling solve failed", e),
    };
    println!("[SERVER] Return value {:#?}", ret_val);
    println!("");

    // Check Solution
    let mut args2: Vec<SuiValue> = Vec::new();
    args2.push(SuiValue::Object(FakeID::Enumerated(5, 0), None));

    let type_args_valid : Vec<TypeTag> = Vec::new();

    // Validate Solution
    let _sol_ret = match suitf.call_function(
        chall_addr,
        "merch_store",
        "has_flag",
        args2,
        type_args_valid,
        Some("solver".to_string()),
    ).await {
        Ok(_output) => {
            println!("[SERVER] Correct Solution!");
            println!("");
            if let Ok(flag) = env::var("FLAG") {
                let message = format!("[SERVER] Congrats, flag: {}", flag);
                stream.write(message.as_bytes()).unwrap();
            } else {
                stream.write("[SERVER] Flag not found, please contact admin".as_bytes()).unwrap();
            }
        }
        Err(e) => handle_err!(stream, "Calling has_flag failed", e),
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
                    let _ = local.run_until( async move {
                        tokio::task::spawn_local( async {
                            if let Err(e) = handle_client(stream).await {
                                eprintln!("[SERVER] Connection Closed. Error: {}", e);
                            }
                        }).await.unwrap();
                    }).await;
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
