# Sui CTF Framework

## Usage

To get started, just add the following line to the `dependencies` section in your `Cargo.toml`:
```toml
[dependencies]
sui-ctf-framework = { git = "https://github.com/otter-sec/sui-ctf-framework" }
```

## Initialize SUI Test Adapter
```rust
let challenge_name = "baby_otter";
let named_addresses = vec![
    (
        "challenge".to_string(),
        NumericalAddress::parse_str(
            "0x8107417ed30fcc2d0b0dfd680f12f6ead218cb971cb989afc8d28ad37da89467",
        )?,
    ),
    (
        "solution".to_string(),
        NumericalAddress::parse_str(
            "0x42f5c1c42496636b461f1cb4f8d62aac8ebac3ca7766c154b63942671bc86836",
        )?,
    ),
];

let precompiled = sui_ctf_framework::get_precompiled(Path::new(&format!(
    "./chall/build/{}/sources/dependencies",
    challenge_name
)));

let mut adapter = sui_ctf_framework::initialize(
    named_addresses,
    &precompiled,
    Some(vec!["challenger".to_string(), "solver".to_string()]),
).await;
```

## Publish Module
```rust
let mod_bytes: Vec<u8> = std::fs::read(format!(
    "./chall/build/{}/bytecode_modules/{}.mv",
    challenge_name, challenge_name
))?;
let chall_dependencies: Vec<String> = Vec::new();
let chall_addr = sui_ctf_framework::publish_compiled_module(
    &mut adapter,
    mod_bytes,
    chall_dependencies,
    Some(String::from("challenger")),
).await;
println!("[SERVER] Challenge published at: {:?}", chall_addr);
```

## Call Contract Function
```rust
let mut args_chall: Vec<SuiValue> = Vec::new();
let arg_obj = SuiValue::Object(FakeID::Enumerated(1, 0), None);
args_chall.push(arg_obj);

let ret_val = sui_ctf_framework::call_function(
    &mut adapter,
    chall_addr,
    challenge_name,
    "function_name",
    args_chall,
    Some("solver".to_string()),
).await;
println!("[SERVER] Return value {:#?}", ret_val);
```