# Sui CTF Framework
A modular framework for running CTF challenges on the Sui blockchain.
It was used during:
- Sui Basecamp CTF 2024 & 2025
- MetaTrust CTF 2023
- JustCTF 2023


To get started, just add the following line to the `dependencies` section in your `Cargo.toml`:
```toml
[dependencies]
sui-ctf-framework = { git = "https://github.com/otter-sec/sui-ctf-framework" }
```

## initialize
Initializes a new Sui test environment with specified named addresses and optional accounts.

**Signature:**
```rust
pub async fn initialize<'a>(
    named_addresses: Vec<(String, NumericalAddress)>,
    accounts: Option<Vec<String>>,
) -> SuiTestAdapter
```

**Example:**
```rust
let named_addresses = vec![
    ("challenge".to_string(), NumericalAddress::parse_str("0x0")?),
    ("solution".to_string(), NumericalAddress::parse_str("0x0")?),
    ("admin".to_string(), NumericalAddress::parse_str("0xfccc9a421bbb13c1a66a1aa98f0ad75029ede94857779c6915b44f94068b921e")?),
];

let mut adapter = sui_ctf_framework::initialize(
    named_addresses,
    Some(vec!["challenger".to_string(), "solver".to_string()]),
).await;
```

## publish_compiled_module
Publishes a compiled module to the Sui network.

**Signature:**
```rust
pub async fn publish_compiled_module(
    adapter: &mut SuiTestAdapter, 
    modules: Vec<MaybeNamedCompiledModule>, 
    module_dependencies: Vec<String>, 
    sender: Option<String>
) -> Option<AccountAddress>
```

**Example:**
```rust
// Publish challenge modules
let chall_dependencies: Vec<String> = Vec::new();
let chall_addr = match sui_ctf_framework::publish_compiled_module(
    &mut adapter,
    mncp_modules,
    chall_dependencies,
    Some(String::from("challenger")),
).await {
    Some(addr) => addr,
    None => {
        // Handle error
        return Ok(());
    }
};

// Publish solution module
let mut sol_dependencies: Vec<String> = Vec::new();
sol_dependencies.push(String::from("challenge"));
let sol_addr = match sui_ctf_framework::publish_compiled_module(
    &mut adapter,
    mncp_solution,
    sol_dependencies,
    Some(String::from("solver")),
).await {
    Some(addr) => addr,
    None => {
        // Handle error
        return Ok(());
    }
};
```

## call_function
Calls a function in a published Move module.

**Signature:**
```rust
pub async fn call_function(
    adapter: &mut SuiTestAdapter,
    mod_addr: AccountAddress,
    mod_name: &str,
    fun_name: &str,
    args: Vec<SuiValue>,
    type_args: Vec<TypeTag>,
    signer: Option<String>,
) -> Result<Option<String>, Box<dyn error::Error>>
```

**Example:**
```rust
// Prepare function arguments
let mut args_liq: Vec<SuiValue> = Vec::new();
let arg_liq1 = SuiValue::Object(FakeID::Enumerated(2, 1), None);
let arg_liq2 = SuiValue::Object(FakeID::Enumerated(2, 5), None);
let arg_liq3 = SuiValue::Object(FakeID::Enumerated(2, 6), None);
args_liq.push(arg_liq1);
args_liq.push(arg_liq2);
args_liq.push(arg_liq3);

// Prepare type arguments
let mut type_args: Vec<TypeTag> = Vec::new();
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

// Call function
let ret_val = match sui_ctf_framework::call_function(
    &mut adapter,
    chall_addr,
    "OtterSwap",
    "initialize_pool",
    args_liq,
    type_args,
    Some("challenger".to_string()),
).await {
    Ok(output) => output,
    Err(e) => {
        // Handle error
        return Err("error during call".into())
    }
};
```

## view_object
Retrieves and parses information about an object on the Sui blockchain.

**Signature:**
```rust
pub async fn view_object(
    adapter: &mut SuiTestAdapter, 
    id: FakeID
) -> Result<Option<serde_json::Value>, Box<dyn error::Error>>
```

**Example:**
```rust
// View an object with ID 0:0
let object_output: Value = match sui_ctf_framework::view_object(
    &mut adapter, 
    FakeID::Enumerated(0, 0)
).await {
    Ok(output) => {
        println!("Object Output: {:#?}", output);
        output.unwrap()
    }
    Err(_error) => {
        // Handle error
        return Err("error when viewing the object".into())
    }
};

// Access object properties
let bytes_str = object_output
    .get("Contents")
    .and_then(|contents| contents.get("id"))
    .and_then(|id| id.get("id"))
    .and_then(|inner_id| inner_id.get("bytes"))
    .and_then(|bytes| bytes.as_str())
    .unwrap();

```

## fund_account
Sends funds to an account.

**Signature:**
```rust
pub async fn fund_account(
    adapter: &mut SuiTestAdapter, 
    sender: String, 
    amount: u64, 
    account_address: String
)
```

**Example:**
```rust
// Fund the solver account with 1000 tokens from the challenger account
sui_ctf_framework::fund_account(
    &mut adapter,
    "challenger".to_string(),
    1000,
    "solver".to_string()
).await;
```