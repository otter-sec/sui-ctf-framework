use std::net::TcpStream;
use std::io::{Read, Write};
use std::str::from_utf8;
use std::{error::Error, fs};
use std::env;
use std::thread;
use std::time::Duration;
use serde_json;


fn main() -> Result<(), Box<dyn Error>> {
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "31337".to_string());

    println!("  - Connecting to server...");
    let mut stream = TcpStream::connect(format!("{}:{}", host, port))?;
    
    // Set read timeout to prevent infinite hanging
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    
    println!("  - Connected to server!");

    // Read welcome message
    let mut welcome_buf = [0u8; 200];
    stream.read(&mut welcome_buf)?;
    println!("  - {}", from_utf8(&welcome_buf)?.trim_matches('\0'));

    // Step 1: Upload Module
    println!("\n=== STEP 1: Upload Module ===");
    
    // Read and display menu, then select option 1
    let mut menu_buf = [0u8; 500];
    let n = stream.read(&mut menu_buf)?;
    println!("  - Menu received");
    
    // Send option 1
    stream.write_all(b"1")?;
    stream.flush()?;
    println!("  - Selected: Upload Module");
    
    // Read module name prompt and send module name
    thread::sleep(Duration::from_millis(100));
    let mut name_prompt = [0u8; 100];
    stream.read(&mut name_prompt)?;
    
    stream.write_all(b"solution")?;
    stream.flush()?;
    println!("  - Module name: solution");
    
    // Read bytecode prompt
    thread::sleep(Duration::from_millis(100));
    let mut bytecode_prompt = [0u8; 100];
    stream.read(&mut bytecode_prompt)?;
    
    // Load and send module bytecode
    let mod_data = fs::read("./solve/build/solution/bytecode_modules/solution.mv")?;
    println!("  - Loaded solution module ({} bytes)", mod_data.len());
    
    stream.write_all(&mod_data)?;
    stream.flush()?;
    
    // Read upload response
    thread::sleep(Duration::from_millis(200));
    let mut upload_response = [0u8; 500];
    let n = stream.read(&mut upload_response)?;
    println!("  - Upload result: {}", from_utf8(&upload_response[..n])?.trim());

    // Step 2: View Object (Challenge state)
    println!("\n=== STEP 2: View Object ===");
    
    // The menu is already displayed, just send option 2
    stream.write_all(b"2")?;
    stream.flush()?;
    println!("  - Selected: View Object");
    
    // Wait for prompt and send first number
    thread::sleep(Duration::from_millis(100));
    let mut prompt_buf = [0u8; 100];
    stream.read(&mut prompt_buf)?;
    
    stream.write_all(b"1")?;
    stream.flush()?;
    println!("  - First number: 1");
    
    // Wait for prompt and send second number
    thread::sleep(Duration::from_millis(100));
    let mut prompt_buf2 = [0u8; 100];
    stream.read(&mut prompt_buf2)?;
    
    stream.write_all(b"0")?;
    stream.flush()?;
    println!("  - Second number: 0");
    
    // Now read the object response
    thread::sleep(Duration::from_millis(200));
    let mut object_response = String::new();
    let mut temp_buf = [0u8; 4096];
    let mut total_read = 0;
    
    println!("  - DEBUG: Reading object response...");
    
    // Read until we get the delimiter
    loop {
        match stream.read(&mut temp_buf) {
            Ok(0) => {
                println!("  - DEBUG: Connection closed after {} bytes", total_read);
                break;
            }
            Ok(n) => {
                total_read += n;
                let chunk = String::from_utf8_lossy(&temp_buf[..n]);
                object_response.push_str(&chunk);
                
                // Check if we've received the delimiter
                if object_response.contains("---END---") {
                    println!("  - DEBUG: Found END delimiter after {} bytes", total_read);
                    
                    // Send acknowledgment
                    stream.write_all(b"ACK")?;
                    stream.flush()?;
                    
                    // Extract and display object data
                    if let Some(start) = object_response.find("[OBJECT]") {
                        if let Some(end) = object_response.find("\n---END---") {
                            let object_data = &object_response[start..end];
                            println!("  - Challenge object state:");
                            println!("{}", object_data);
                            
                            // Parse JSON to verify it's valid
                            if let Some(json_start) = object_data.find('{') {
                                let json_str = &object_data[json_start..];
                                match serde_json::from_str::<serde_json::Value>(json_str) {
                                    Ok(json) => {
                                        if let Some(contents) = json.get("contents") {
                                            println!("  - Object contents found: {} fields", contents.as_object().map(|o| o.len()).unwrap_or(0));
                                        }
                                    }
                                    Err(e) => println!("  - Warning: Could not parse JSON: {}", e),
                                }
                            }
                        }
                    }
                    
                    // Now read and discard the menu that follows
                    thread::sleep(Duration::from_millis(100));
                    let mut menu_buf = [0u8; 500];
                    match stream.read(&mut menu_buf) {
                        Ok(n) => println!("  - DEBUG: Discarded {} bytes of menu", n),
                        Err(_) => {},
                    }
                    
                    break;
                }
            }
            Err(e) => {
                println!("  - DEBUG: Read error after {} bytes: {}", total_read, e);
                return Err(e.into());
            }
        }
    }

    // Step 3: Call Functions (Complete the challenge step by step)
    println!("\n=== STEP 3: Execute Solution Functions ===");
    
    // Step 3a: Call solve_step_one (UserProgress is at 2,0)
    println!("  - Calling solve_step_one...");
    call_function(&mut stream, "solution", "solve_step_one", vec![("object", "2", "0")])?;
    
    // Step 3b: Call solve_step_two  
    println!("  - Calling solve_step_two...");
    call_function(&mut stream, "solution", "solve_step_two", vec![("object", "2", "0"), ("object", "1", "0")])?;
    
    // Step 3c: Call solve_step_three
    println!("  - Calling solve_step_three...");
    call_function(&mut stream, "solution", "solve_step_three", vec![("object", "2", "0"), ("object", "1", "0")])?;
    
    // Step 3d: Call complete_challenge
    println!("  - Calling complete_challenge...");
    call_function(&mut stream, "solution", "complete_challenge", vec![("object", "2", "0"), ("object", "1", "0")])?;

    // Step 4: Get Flag
    println!("\n=== STEP 4: Get Flag ===");
    
    // Add delay and send option 4 directly
    thread::sleep(Duration::from_millis(300));
    stream.write_all(b"4")?;
    stream.flush()?;
    println!("  - Selected: Get Flag");
    
    // Read flag response
    thread::sleep(Duration::from_millis(200));
    let mut flag_response = [0u8; 500];
    let n = stream.read(&mut flag_response)?;
    println!("  - Flag result: {}", from_utf8(&flag_response[..n])?.trim());

    // Step 5: Exit
    println!("\n=== STEP 5: Exit ===");
    
    // Read the menu first, then send option 5
    thread::sleep(Duration::from_millis(200));
    let mut menu_buf = [0u8; 200];
    stream.read(&mut menu_buf)?;
    
    stream.write_all(b"5")?;
    stream.flush()?;
    println!("  - Selected: Exit");
    
    // Read goodbye message
    thread::sleep(Duration::from_millis(100));
    let mut goodbye = [0u8; 100];
    let n = stream.read(&mut goodbye)?;
    println!("  - Server response: {}", from_utf8(&goodbye[..n])?.trim());

    println!("  - Challenge completed successfully!");
    Ok(())
}

fn call_function(
    stream: &mut TcpStream, 
    module: &str, 
    function: &str, 
    params: Vec<(&str, &str, &str)>
) -> Result<(), Box<dyn Error>> {
    // Add delay to avoid timing issues
    thread::sleep(Duration::from_millis(300));
    
    // Send option 3 for Call Function
    stream.write_all(b"3")?;
    stream.flush()?;
    println!("    - Selected: Call Function");
    
    // Read module name prompt and send module name
    thread::sleep(Duration::from_millis(100));
    let mut prompt_buf = [0u8; 100];
    stream.read(&mut prompt_buf)?;
    
    stream.write_all(module.as_bytes())?;
    stream.flush()?;
    println!("    - Module: {}", module);
    
    // Read function name prompt and send function name
    thread::sleep(Duration::from_millis(100));
    let mut prompt_buf2 = [0u8; 100];
    stream.read(&mut prompt_buf2)?;
    
    stream.write_all(function.as_bytes())?;
    stream.flush()?;
    println!("    - Function: {}", function);
    
    // Read parameter count prompt and send parameter count
    thread::sleep(Duration::from_millis(100));
    let mut prompt_buf3 = [0u8; 100];
    stream.read(&mut prompt_buf3)?;
    
    let param_count = params.len().to_string();
    stream.write_all(param_count.as_bytes())?;
    stream.flush()?;
    println!("    - Parameters: {}", param_count);
    
    // Send each parameter
    for (i, (param_type, num1, num2)) in params.iter().enumerate() {
        println!("    - Parameter {}: {} ({}, {})", i + 1, param_type, num1, num2);
        
        // Read parameter type prompt and send parameter type
        thread::sleep(Duration::from_millis(100));
        let mut type_prompt_buf = [0u8; 100];
        stream.read(&mut type_prompt_buf)?;
        
        stream.write_all(param_type.as_bytes())?;
        stream.flush()?;
        
        if *param_type == "object" {
            // Read first number prompt and send first number
            thread::sleep(Duration::from_millis(100));
            let mut num1_prompt_buf = [0u8; 100];
            stream.read(&mut num1_prompt_buf)?;
            
            stream.write_all(num1.as_bytes())?;
            stream.flush()?;
            
            // Read second number prompt and send second number
            thread::sleep(Duration::from_millis(100));
            let mut num2_prompt_buf = [0u8; 100];
            stream.read(&mut num2_prompt_buf)?;
            
            stream.write_all(num2.as_bytes())?;
            stream.flush()?;
        }
        // Add handling for other parameter types if needed
    }
    
    // Read function result
    thread::sleep(Duration::from_millis(200));
    let mut result = [0u8; 1000];
    let n = stream.read(&mut result)?;
    let result_text = from_utf8(&result[..n])?.trim();
    println!("    - Result: {}", result_text);
    
    // Read and discard the menu that follows
    thread::sleep(Duration::from_millis(100));
    let mut menu_buf = [0u8; 500];
    match stream.read(&mut menu_buf) {
        Ok(n) => println!("    - DEBUG: Discarded {} bytes of menu", n),
        Err(_) => {},
    }
    
    Ok(())
}