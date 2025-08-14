use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::io::{Read, Write};
use std::mem::drop;
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::{thread, time::Duration};

use serde_json;
use tokio;

use move_transactional_test_runner::framework::MaybeNamedCompiledModule;
use move_bytecode_source_map::{source_map::SourceMap, utils::source_map_from_file};
use move_binary_format::file_format::CompiledModule;
use move_symbol_pool::Symbol;
use move_core_types::{
    account_address::AccountAddress, 
    language_storage::TypeTag,
    runtime_value::MoveValue};

use sui_ctf_framework::NumericalAddress;
use sui_transactional_test_runner::{args::SuiValue, test_adapter::FakeID};

macro_rules! handle_err {
    ($stream:expr, $msg:expr, $err:expr) => {{
        let full = format!("[SERVER ERROR] {}: {}", $msg, $err);
        eprintln!("{}", full);
        let _ = $stream.write_all(full.as_bytes());   // ignore write failures
        let _ = $stream.write_all(b"\n[SERVER] Connection will be closed due to error.\n");
        let _ = $stream.flush();
        drop($stream);                                // close socket
        return Err::<(), Box<dyn std::error::Error>>(full.into());
    }};
}

macro_rules! handle_input_error {
    ($stream:expr, $msg:expr) => {{
        let err_msg = format!("[ERROR] {}\n", $msg);
        eprintln!("{}", err_msg);
        if let Err(e) = $stream.write_all(err_msg.as_bytes()) {
            eprintln!("[SERVER] Failed to send error message: {}", e);
            return Err(e.into());
        }
        let _ = $stream.flush();
    }};
}

macro_rules! read_input_with_timeout {
    ($stream:expr, $buf:expr, $timeout_msg:expr) => {{
        match $stream.read($buf) {
            Ok(n) if n > 0 => n,
            Ok(0) => {
                eprintln!("[SERVER] Client disconnected");
                return Ok(());
            }
            Err(e) => {
                eprintln!("[SERVER] Read error: {}", e);
                handle_input_error!($stream, $timeout_msg);
                return Err(e.into());
            }
        }
    }};
}

async fn handle_client(mut stream: TcpStream) -> Result<(), Box<dyn Error>> {
    // Set connection timeouts to prevent hanging
    stream.set_read_timeout(Some(std::time::Duration::from_secs(300)))?; // 5 minutes
    stream.set_write_timeout(Some(std::time::Duration::from_secs(30)))?; // 30 seconds
    
    println!("[SERVER] Client connected with timeouts set");
    
    // Initialize SuiTestAdapter
    let module_name = "interactive_ctf";
    let mut deployed_modules: Vec<AccountAddress> = Vec::new();
    let mut module_name_to_address: HashMap<String, AccountAddress> = HashMap::new();

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
    ];

    let mut suitf = match sui_ctf_framework::SuiTF::initialize(
        named_addresses,
        Some(vec!["challenger".to_string(), "solver".to_string()]),
    ).await {
        Ok(adapter) => adapter,
        Err(e) => handle_err!(stream, "SuiTF initialization failed", e),
    };

    // Publish challenge module
    let mut mncp_modules : Vec<MaybeNamedCompiledModule> = Vec::new();
    let mod_path = format!("./chall/build/challenge/bytecode_modules/{}.mv", module_name);
    let src_path = format!("./chall/build/challenge/debug_info/{}.json", module_name);
    let mod_bytes: Vec<u8> = match std::fs::read(mod_path) {
        Ok(data) => data,
        Err(e) => handle_err!(stream, format!("Failed to read {}", module_name), e),
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
    module_name_to_address.insert("challenge".to_string(), chall_addr);
    println!("[SERVER] Module published at: {:?}", chall_addr);
    
    // Create UserProgress for the solver
    let mut create_progress_args: Vec<SuiValue> = Vec::new(); // No args needed
    let create_progress_type_args: Vec<TypeTag> = Vec::new();
    
    match suitf.call_function(
        chall_addr,
        "interactive_ctf",
        "create_progress",
        create_progress_args,
        create_progress_type_args,
        Some("solver".to_string()),
    ).await {
        Ok(_) => {
            println!("[SERVER] UserProgress created for solver");
        }
        Err(e) => handle_err!(stream, "Failed to create UserProgress", e),
    }; 

    // Send welcome message
    let welcome_msg = "[SERVER] Welcome to the Interactive CTF Challenge!\n";
    stream.write_all(welcome_msg.as_bytes())?;

    // Interactive menu loop
    loop {
        // Send menu
        let menu = "\n[MENU]\n1. Upload Module\n2. View Object\n3. Call Function\n4. Get Flag\n5. Exit\nSelect option: ";
        stream.write_all(menu.as_bytes())?;
        stream.flush()?;

        // Read user choice with timeout handling
        let mut choice_buf = [0u8; 10];
        let n = read_input_with_timeout!(stream, &mut choice_buf, "Connection timed out waiting for menu choice");
        
        let choice = String::from_utf8_lossy(&choice_buf[..n]).trim().to_string();
        
        // Validate choice
        if choice.is_empty() {
            handle_input_error!(stream, "Empty input received. Please enter a valid option (1-5)");
            continue;
        }
        
        match choice.as_str() {
            "1" => {
                // Upload Module
                stream.write_all(b"Enter module name for named address: ")?;
                stream.flush()?;
                
                let mut name_buf = [0u8; 100];
                let n = read_input_with_timeout!(stream, &mut name_buf, "Timeout waiting for module name");
                let module_name = String::from_utf8_lossy(&name_buf[..n]).trim().to_string();
                
                if module_name.is_empty() {
                    handle_input_error!(stream, "Module name cannot be empty");
                    continue;
                }
                
                if module_name.len() > 50 {
                    handle_input_error!(stream, "Module name too long (max 50 characters)");
                    continue;
                }
                
                stream.write_all(b"Send module bytecode (max 2000 bytes): ")?;
                stream.flush()?;
                
                let mut module_data = [0u8; 2000];
                let module_size = read_input_with_timeout!(stream, &mut module_data, "Timeout waiting for module bytecode");
                
                if module_size == 0 {
                    handle_input_error!(stream, "No module bytecode received");
                    continue;
                }
                
                // Publish module
                let mut sol_dependencies: Vec<String> = Vec::new();
                sol_dependencies.push(String::from("challenge"));

                let mut mncp_solution : Vec<MaybeNamedCompiledModule> = Vec::new();
                let module : CompiledModule = match CompiledModule::deserialize_with_defaults(&module_data[..module_size].to_vec()) {
                    Ok(m) => m,
                    Err(e) => {
                        let err_msg = format!("[ERROR] Module deserialization failed: {}\n", e);
                        stream.write_all(err_msg.as_bytes())?;
                        continue;
                    }
                };
                let named_addr_opt: Option<Symbol> = Some(Symbol::from(module_name.as_str()));
                let source_map : Option<SourceMap> = None;
                
                mncp_solution.push( MaybeNamedCompiledModule {
                    named_address: named_addr_opt,
                    module: module,
                    source_map: source_map,
                });

                match suitf.publish_compiled_module(
                    mncp_solution,
                    sol_dependencies,
                    Some(String::from("solver")),
                ).await {
                    Ok(addr) => {
                        module_name_to_address.insert(module_name.clone(), addr);
                        let success_msg = format!("[SUCCESS] Module '{}' published at: {}\n", module_name, addr);
                        stream.write_all(success_msg.as_bytes())?;
                    }
                    Err(e) => {
                        let err_msg = format!("[ERROR] Module publish failed: {}\n", e);
                        stream.write_all(err_msg.as_bytes())?;
                    }
                };
            }
            "2" => {
                // View Object
                stream.write_all(b"Enter first number for object ID: ")?;
                stream.flush()?;
                
                let mut num1_buf = [0u8; 20];
                let n = read_input_with_timeout!(stream, &mut num1_buf, "Timeout waiting for first object number");
                let num1_str = String::from_utf8_lossy(&num1_buf[..n]).trim();
                
                if num1_str.is_empty() {
                    handle_input_error!(stream, "First number cannot be empty");
                    continue;
                }
                
                let num1: u64 = match num1_str.parse() {
                    Ok(n) => n,
                    Err(_) => {
                        handle_input_error!(stream, format!("Invalid first number: '{}'. Please enter a valid integer", num1_str));
                        continue;
                    }
                };
                
                stream.write_all(b"Enter second number for object ID: ")?;
                stream.flush()?;
                
                let mut num2_buf = [0u8; 20];
                let n = read_input_with_timeout!(stream, &mut num2_buf, "Timeout waiting for second object number");
                let num2_str = String::from_utf8_lossy(&num2_buf[..n]).trim();
                
                if num2_str.is_empty() {
                    handle_input_error!(stream, "Second number cannot be empty");
                    continue;
                }
                
                let num2: u64 = match num2_str.parse() {
                    Ok(n) => n,
                    Err(_) => {
                        handle_input_error!(stream, format!("Invalid second number: '{}'. Please enter a valid integer", num2_str));
                        continue;
                    }
                };
                
                // View object
                match suitf.view_object(FakeID::Enumerated(num1, num2)).await {
                    Ok(Some(output)) => {
                        println!("[SERVER] Object view returned data: {:#?}", output);
                        let output_str = format!("[OBJECT] {}\n", serde_json::to_string_pretty(&output)?);
                        println!("[SERVER] Sending object response: {}", output_str);
                        stream.write_all(output_str.as_bytes())?;
                        stream.write_all(b"\n---END---\n")?; // Add delimiter
                        stream.flush()?;
                        println!("[SERVER] Object response sent and flushed");
                        
                        // Wait for client acknowledgment before continuing
                        let mut ack_buf = [0u8; 10];
                        match stream.read(&mut ack_buf) {
                            Ok(_) => println!("[SERVER] Client acknowledged object response"),
                            Err(e) => println!("[SERVER] Warning: No client ack: {}", e),
                        }
                    }
                    Ok(None) => {
                        println!("[SERVER] Object view returned None");
                        stream.write_all(b"[OBJECT] No output\n")?;
                        stream.flush()?;
                    }
                    Err(e) => {
                        println!("[SERVER] Object view error: {:?}", e);
                        let err_msg = format!("[ERROR] Failed to view object: {}\n", e);
                        stream.write_all(err_msg.as_bytes())?;
                        stream.flush()?;
                    }
                }
            }
            "3" => {
                // Call Function
                stream.write_all(b"Enter module name: ")?;
                stream.flush()?;
                
                let mut mod_name_buf = [0u8; 100];
                let n = read_input_with_timeout!(stream, &mut mod_name_buf, "Timeout waiting for module name");
                let mod_name = String::from_utf8_lossy(&mod_name_buf[..n]).trim().to_string();
                
                if mod_name.is_empty() {
                    handle_input_error!(stream, "Module name cannot be empty");
                    continue;
                }
                
                let mod_addr = match module_name_to_address.get(&mod_name) {
                    Some(addr) => *addr,
                    None => {
                        handle_input_error!(stream, format!("Module '{}' not found. Available modules: {}", mod_name, 
                            module_name_to_address.keys().collect::<Vec<_>>().join(", ")));
                        continue;
                    }
                };
                
                stream.write_all(b"Enter function name: ")?;
                stream.flush()?;
                
                let mut func_name_buf = [0u8; 100];
                let n = read_input_with_timeout!(stream, &mut func_name_buf, "Timeout waiting for function name");
                let func_name = String::from_utf8_lossy(&func_name_buf[..n]).trim().to_string();
                
                if func_name.is_empty() {
                    handle_input_error!(stream, "Function name cannot be empty");
                    continue;
                }
                
                stream.write_all(b"Enter number of parameters: ")?;
                stream.flush()?;
                
                let mut param_count_buf = [0u8; 10];
                let n = read_input_with_timeout!(stream, &mut param_count_buf, "Timeout waiting for parameter count");
                let param_count_str = String::from_utf8_lossy(&param_count_buf[..n]).trim();
                
                if param_count_str.is_empty() {
                    handle_input_error!(stream, "Parameter count cannot be empty");
                    continue;
                }
                
                let param_count: usize = match param_count_str.parse() {
                    Ok(n) if n <= 10 => n, // Reasonable limit
                    Ok(n) => {
                        handle_input_error!(stream, format!("Too many parameters: {}. Maximum allowed is 10", n));
                        continue;
                    }
                    Err(_) => {
                        handle_input_error!(stream, format!("Invalid parameter count: '{}'. Please enter a valid number", param_count_str));
                        continue;
                    }
                };
                
                let mut args: Vec<SuiValue> = Vec::new();
                
                for i in 0..param_count {
                    let param_msg = format!("Parameter {} - Enter type (number/list/object): ", i + 1);
                    stream.write_all(param_msg.as_bytes())?;
                    stream.flush()?;
                    
                    let mut type_buf = [0u8; 20];
                    let n = read_input_with_timeout!(stream, &mut type_buf, "Timeout waiting for parameter type");
                    let param_type = String::from_utf8_lossy(&type_buf[..n]).trim().to_string();
                    
                    if param_type.is_empty() {
                        handle_input_error!(stream, "Parameter type cannot be empty");
                        continue;
                    }
                    
                    match param_type.as_str() {
                        "number" => {
                            stream.write_all(b"Enter number type (u8/u16/u32/u64): ")?;
                            stream.flush()?;
                            
                            let mut num_type_buf = [0u8; 10];
                            let n = stream.read(&mut num_type_buf)?;
                            let num_type = String::from_utf8_lossy(&num_type_buf[..n]).trim().to_string();
                            
                            stream.write_all(b"Enter value: ")?;
                            stream.flush()?;
                            
                            let mut value_buf = [0u8; 20];
                            let n = stream.read(&mut value_buf)?;
                            let value: u64 = match String::from_utf8_lossy(&value_buf[..n]).trim().parse() {
                                Ok(n) => n,
                                Err(_) => {
                                    stream.write_all(b"[ERROR] Invalid value\n")?;
                                    continue;
                                }
                            };
                            
                            match num_type.as_str() {
                                "u8" => args.push(SuiValue::MoveValue(MoveValue::U8(value as u8))),
                                "u16" => args.push(SuiValue::MoveValue(MoveValue::U16(value as u16))),
                                "u32" => args.push(SuiValue::MoveValue(MoveValue::U32(value as u32))),
                                "u64" => args.push(SuiValue::MoveValue(MoveValue::U64(value))),
                                _ => {
                                    stream.write_all(b"[ERROR] Invalid number type\n")?;
                                    continue;
                                }
                            }
                        }
                        "list" => {
                            stream.write_all(b"Enter list length: ")?;
                            stream.flush()?;
                            
                            let mut len_buf = [0u8; 10];
                            let n = stream.read(&mut len_buf)?;
                            let len: usize = match String::from_utf8_lossy(&len_buf[..n]).trim().parse() {
                                Ok(n) => n,
                                Err(_) => {
                                    stream.write_all(b"[ERROR] Invalid length\n")?;
                                    continue;
                                }
                            };
                            
                            let mut list_values = Vec::new();
                            for j in 0..len {
                                let elem_msg = format!("Enter element {} (u8): ", j);
                                stream.write_all(elem_msg.as_bytes())?;
                                stream.flush()?;
                                
                                let mut elem_buf = [0u8; 10];
                                let n = stream.read(&mut elem_buf)?;
                                let elem: u8 = match String::from_utf8_lossy(&elem_buf[..n]).trim().parse() {
                                    Ok(n) => n,
                                    Err(_) => {
                                        stream.write_all(b"[ERROR] Invalid element\n")?;
                                        continue;
                                    }
                                };
                                list_values.push(elem);
                            }
                            let move_values: Vec<MoveValue> = list_values.into_iter().map(MoveValue::U8).collect();
                            args.push(SuiValue::MoveValue(MoveValue::Vector(move_values)));
                        }
                        "object" => {
                            stream.write_all(b"Enter first number for object ID: ")?;
                            stream.flush()?;
                            
                            let mut num1_buf = [0u8; 20];
                            let n = read_input_with_timeout!(stream, &mut num1_buf, "Timeout waiting for first object number");
                            let num1_str = String::from_utf8_lossy(&num1_buf[..n]).trim();
                            
                            if num1_str.is_empty() {
                                handle_input_error!(stream, "First object number cannot be empty");
                                continue;
                            }
                            
                            let num1: u64 = match num1_str.parse() {
                                Ok(n) => n,
                                Err(_) => {
                                    handle_input_error!(stream, format!("Invalid first object number: '{}'", num1_str));
                                    continue;
                                }
                            };
                            
                            stream.write_all(b"Enter second number for object ID: ")?;
                            stream.flush()?;
                            
                            let mut num2_buf = [0u8; 20];
                            let n = read_input_with_timeout!(stream, &mut num2_buf, "Timeout waiting for second object number");
                            let num2_str = String::from_utf8_lossy(&num2_buf[..n]).trim();
                            
                            if num2_str.is_empty() {
                                handle_input_error!(stream, "Second object number cannot be empty");
                                continue;
                            }
                            
                            let num2: u64 = match num2_str.parse() {
                                Ok(n) => n,
                                Err(_) => {
                                    handle_input_error!(stream, format!("Invalid second object number: '{}'", num2_str));
                                    continue;
                                }
                            };
                            
                            args.push(SuiValue::Object(FakeID::Enumerated(num1, num2), None));
                        }
                        _ => {
                            handle_input_error!(stream, format!("Invalid parameter type: '{}'. Valid types are: number, list, object", param_type));
                            continue;
                        }
                    }
                }
                
                // Call function
                let type_args: Vec<TypeTag> = Vec::new(); // Simplified for now
                
                // Determine the actual module name based on the address
                let actual_module_name = if mod_name == "challenge" {
                    "interactive_ctf"
                } else {
                    "solution"  // Default for user modules
                };
                
                match suitf.call_function(
                    mod_addr,
                    actual_module_name,
                    &func_name,
                    args,
                    type_args,
                    Some("solver".to_string()),
                ).await {
                    Ok(Some(output)) => {
                        let output_msg = format!("[SUCCESS] Function output: {}\n", output);
                        stream.write_all(output_msg.as_bytes())?;
                    }
                    Ok(None) => {
                        stream.write_all(b"[SUCCESS] Function executed (no output)\n")?;
                    }
                    Err(e) => {
                        let err_msg = format!("[ERROR] Function call failed: {}\n", e);
                        stream.write_all(err_msg.as_bytes())?;
                    }
                }
            }
            "4" => {
                // Get Flag - check if challenge is solved
                let mut args: Vec<SuiValue> = Vec::new();
                args.push(SuiValue::Object(FakeID::Enumerated(1, 0), None)); // Challenge object
                
                let type_args: Vec<TypeTag> = Vec::new();
                
                match suitf.call_function(
                    chall_addr,
                    "interactive_ctf",
                    "check_solution",
                    args,
                    type_args,
                    Some("solver".to_string()),
                ).await {
                    Ok(_) => {
                        if let Ok(flag) = env::var("FLAG") {
                            let message = format!("[FLAG] Congrats! Flag: {}\n", flag);
                            stream.write_all(message.as_bytes())?;
                        } else {
                            stream.write_all(b"[FLAG] Flag not found, please contact admin\n")?;
                        }
                    }
                    Err(e) => {
                        let err_msg = format!("[ERROR] Solution check failed: {}\n", e);
                        stream.write_all(err_msg.as_bytes())?;
                    }
                }
            }
            "5" => {
                // Exit
                stream.write_all(b"[SERVER] Goodbye!\n")?;
                break;
            }
            _ => {
                handle_input_error!(stream, format!("Invalid option: '{}'. Please select 1-5", choice));
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create Socket - Port 31337
    let listener = TcpListener::bind("0.0.0.0:31337")?;
    println!("[SERVER] Starting interactive server at port 31337!");

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