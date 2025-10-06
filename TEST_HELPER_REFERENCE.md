# Test Helper Reference

**Last Updated**: After implementing name encoding and validation
**File**: `tests/test_helper.rs`

## Overview

The `RegistryTestHelper` provides a comprehensive test infrastructure for the Miden ID registry contract. It handles client setup, contract deployment, account management, name registration, and query operations.

---

## Data Structures

### `ContractState`
Complete contract state structure for validation and debugging.

**Fields**:
```rust
pub struct ContractState {
    pub initialized: u64,           // Slot 0: Initialization flag
    pub owner_prefix: u64,           // Slot 1: Owner account prefix
    pub owner_suffix: u64,           // Slot 1: Owner account suffix
    pub token_prefix: u64,           // Slot 2: Payment token prefix
    pub token_suffix: u64,           // Slot 2: Payment token suffix
    pub name_to_id_mapping: Option<Word>,  // Slot 3 root hash
    pub id_to_name_mapping: Option<Word>,  // Slot 4 root hash
}
```

---

### `RegistryTestHelper`
Main test helper struct encapsulating all test operations.

**Fields**:
```rust
pub struct RegistryTestHelper {
    pub client: Client,                    // Miden client instance
    pub endpoint: Endpoint,                // Network endpoint
    pub keystore: FilesystemKeyStore,      // Key storage
    pub registry_contract: Option<Account>, // Deployed registry
    pub owner_account: Option<Account>,     // Registry owner
}
```

---

## Setup & Lifecycle Methods

### `new()`
**Signature**: `pub async fn new() -> Result<Self, ClientError>`
**Purpose**: Create a new test helper with network connection
**Returns**: Configured `RegistryTestHelper` instance

**Behavior**:
1. Deletes existing keystore and store (clean slate)
2. Attempts testnet connection
3. Falls back to localhost on failure
4. Initializes filesystem keystore at `./keystore`

**Example**:
```rust
let mut helper = RegistryTestHelper::new().await?;
```

---

### `setup_with_deployed_contract()`
**Signature**: `pub async fn setup_with_deployed_contract() -> Result<Self, ClientError>`
**Purpose**: Create helper AND deploy registry contract (all-in-one setup)
**Returns**: Helper with deployed registry ready to initialize

**Behavior**:
1. Calls `new()` to setup client
2. Deploys registry contract
3. Returns helper ready for `initialize_registry()`

**Example**:
```rust
let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
let owner = helper.create_account("Owner").await?;
helper.initialize_registry(&owner).await?;
```

---

### `sync_network()`
**Signature**: `pub async fn sync_network(&mut self) -> Result<(), ClientError>`
**Purpose**: Sync with network and get latest state
**Returns**: `Ok(())` on success

**Usage**: Call after transactions to ensure state is updated

---

## Account Management

### `create_account()`
**Signature**: `pub async fn create_account(&mut self, _role: &str) -> Result<Account, ClientError>`
**Purpose**: Create a new basic wallet account
**Parameters**:
- `_role`: Descriptive name (not used functionally, just for debugging)

**Returns**: New `Account` instance
**Delay**: Sleeps 3 seconds after creation for network propagation

**Example**:
```rust
let user = helper.create_account("User").await?;
let owner = helper.create_account("Owner").await?;
```

---

## Contract Deployment & Initialization

### `deploy_registry_contract()`
**Signature**: `pub async fn deploy_registry_contract(&mut self) -> Result<Account, ClientError>`
**Purpose**: Deploy the Miden ID registry contract
**Returns**: Deployed contract account

**Behavior**:
1. Reads `./masm/accounts/miden_id.masm`
2. Creates public immutable contract
3. Stores contract in `self.registry_contract`
4. Syncs network state

**Note**: Contract is deployed but NOT initialized. Call `initialize_registry()` next.

---

### `initialize_registry()`
**Signature**: `pub async fn initialize_registry(&mut self, owner: &Account) -> Result<(), ClientError>`
**Purpose**: Initialize the deployed registry contract
**Parameters**:
- `owner`: Account that will own the registry

**Behavior**:
1. Creates initialization note with:
   - Price: `[100, 0, 0, 0, 0]` (5 elements)
   - Payment token: From owner account ID
2. Executes initialization transaction
3. Waits 15 seconds for confirmation
4. Syncs network
5. Stores owner in `self.owner_account`

**Note**: Must call `deploy_registry_contract()` first

**Example**:
```rust
helper.deploy_registry_contract().await?;
let owner = helper.create_account("Owner").await?;
helper.initialize_registry(&owner).await?;
```

---

## State Query Methods

### `get_contract_state()`
**Signature**: `pub async fn get_contract_state(&mut self) -> Result<Option<AccountRecord>, ClientError>`
**Purpose**: Get current contract account record
**Returns**: `Option<AccountRecord>` with contract state

**Usage**: Access storage slots, verify state changes

---

### `get_initialization_state()`
**Signature**: `pub fn get_initialization_state(&self, account_record: &AccountRecord) -> (u64, u64, u64)`
**Purpose**: Extract initialization state from slot 0
**Returns**: `(initialized_flag, 0, 0)` tuple

---

### `get_payment_token_state()`
**Signature**: `pub fn get_payment_token_state(&self, account_record: &AccountRecord) -> (u64, u64)`
**Purpose**: Extract payment token from slot 2
**Returns**: `(token_prefix, token_suffix)` tuple

---

### `get_registry_mapping_state()`
**Signature**: `pub fn get_registry_mapping_state(&self, account_record: &AccountRecord) -> (Option<Word>, Option<Word>)`
**Purpose**: Extract map root hashes from slots 3 & 4
**Returns**: `(slot3_root, slot4_root)` tuple
- `slot3_root`: Name→ID mapping root
- `slot4_root`: ID→Name mapping root

---

### `get_complete_contract_state()`
**Signature**: `pub fn get_complete_contract_state(&self, account_record: &AccountRecord) -> ContractState`
**Purpose**: Extract all contract state into structured format
**Returns**: Complete `ContractState` struct

**Fields Extracted**:
- Slot 0: `initialized`
- Slot 1: `owner_prefix`, `owner_suffix`
- Slot 2: `token_prefix`, `token_suffix`
- Slot 3: `name_to_id_mapping` (root hash)
- Slot 4: `id_to_name_mapping` (root hash)

---

## Name Encoding/Decoding

### Name Encoding Format (POC)
```
Word = [length, chars_1-7, chars_8-14, chars_15-20]

- Felt[0]: Name length (0-20)
- Felt[1]: ASCII characters 1-7 (56 bits used, 7 bits padding)
- Felt[2]: ASCII characters 8-14 (56 bits used, 7 bits padding)
- Felt[3]: ASCII characters 15-20 (56 bits used, 7 bits padding)
```

---

### `encode_name_to_word()`
**Signature**: `pub fn encode_name_to_word(name: &str) -> Word`
**Purpose**: Convert name string to Word encoding
**Parameters**:
- `name`: Name string (max 20 characters)

**Returns**: Encoded `Word([length, chars1-7, chars8-14, chars15-20])`

**Example**:
```rust
let word = RegistryTestHelper::encode_name_to_word("alice");
// Returns: Word([5, 435459550305, 0, 0])
// MASM: push.5.435459550305.0.0
```

**Encoding Details**:
- Felt[0] = name length
- Felt[1-3] = 7 ASCII chars each, packed little-endian
- Example "alice" (5 chars):
  - Felt[0] = 5
  - Felt[1] = 0x65_63_69_6C_61 = 435459550305 (reversed "alice")
  - Felt[2] = 0
  - Felt[3] = 0

**Panic**: Asserts name length <= 20 characters

---

### `decode_name_word()`
**Signature**: `pub fn decode_name_word(word: &Word) -> String`
**Purpose**: Convert encoded Word back to name string
**Parameters**:
- `word`: Encoded name as Word

**Returns**: Decoded name string

**Logic**:
1. Reads length from Felt[0]
2. Extracts ASCII bytes from Felt[1-3] (7 bytes each)
3. Stops after reading `length` characters
4. Converts bytes to UTF-8 string

**Example**:
```rust
let word = Word::new([Felt::new(5), Felt::new(435459550305), Felt::ZERO, Felt::ZERO]);
let name = RegistryTestHelper::decode_name_word(&word);
// Returns: "alice"
```

---

### `is_zero_word()`
**Signature**: `pub fn is_zero_word(word: &Word) -> bool`
**Purpose**: Check if Word is all zeros
**Returns**: `true` if all felts are zero, `false` otherwise

**Usage**: Detect empty/unregistered entries in storage maps

---

## Name Registration

### `register_name_for_account()`
**Signature**: `pub async fn register_name_for_account(&mut self, account: &Account, name: &str) -> Result<(), ClientError>`
**Purpose**: Register a name for an account
**Parameters**:
- `account`: Account to register name for
- `name`: Name to register (max 20 chars)

**Behavior**:
1. Encodes name to Word
2. Generates MASM push string
3. Creates registration note script:
   ```masm
   use.miden_id::registry
   use.std::sys

   begin
       push.{encoded_name}
       call.registry::register_name
       exec.sys::truncate_stack
   end
   ```
4. Creates public note with registry contract library
5. Waits 5 seconds
6. Executes transaction with nop script
7. Submits transaction
8. Waits 15 seconds for confirmation
9. Syncs network

**Validations** (contract-side):
- ✅ Name length <= 20 characters
- ✅ Name not already registered
- ✅ Account doesn't already have a name

**Example**:
```rust
helper.register_name_for_account(&user_account, "alice").await?;
```

---

## Query Functions

### `is_name_registered()`
**Signature**: `pub async fn is_name_registered(&mut self, name: &str) -> Result<bool, ClientError>`
**Purpose**: Check if a name is registered (slot 3 query)
**Parameters**:
- `name`: Name to check

**Returns**: `true` if name exists in slot 3, `false` otherwise

**Implementation**: Queries slot 3 map with encoded name as key

---

### `get_account_for_name()`
**Signature**: `pub async fn get_account_for_name(&mut self, name: &str) -> Result<Option<(u64, u64)>, ClientError>`
**Purpose**: Forward lookup - get account ID for a name (slot 3)
**Parameters**:
- `name`: Name to lookup

**Returns**: `Some((prefix, suffix))` if registered, `None` otherwise

**Example**:
```rust
let (prefix, suffix) = helper.get_account_for_name("alice").await?.unwrap();
```

---

### `has_name_for_address()`
**Signature**: `pub async fn has_name_for_address(&mut self, account: &Account) -> Result<bool, ClientError>`
**Purpose**: Check if an address has a name (slot 4 query)
**Parameters**:
- `account`: Account to check

**Returns**: `true` if account has name in slot 4, `false` otherwise

---

### `get_name_for_address()`
**Signature**: `pub async fn get_name_for_address(&mut self, account: &Account) -> Result<Option<String>, ClientError>`
**Purpose**: Reverse lookup - get name for an address (slot 4)
**Parameters**:
- `account`: Account to lookup

**Returns**: `Some(name_string)` if registered, `None` otherwise

**Implementation**:
1. Gets name Word from slot 4
2. Decodes Word to string using `decode_name_word()`

**Example**:
```rust
let name = helper.get_name_for_address(&user_account).await?;
// Returns: Some("alice")
```

---

### `get_account_word_for_name()`
**Signature**: `pub async fn get_account_word_for_name(&mut self, name: &str) -> Result<Option<Word>, ClientError>`
**Purpose**: Get raw Word value from slot 3 for a name
**Parameters**:
- `name`: Name to lookup

**Returns**: Raw `Word` from slot 3 map (account ID as `[0,0,prefix,suffix]`)

**Usage**: Low-level testing, usually use `get_account_for_name()` instead

---

### `get_name_word_for_account()`
**Signature**: `pub async fn get_name_word_for_account(&mut self, account: &Account) -> Result<Option<Word>, ClientError>`
**Purpose**: Get raw Word value from slot 4 for an account
**Parameters**:
- `account`: Account to lookup

**Returns**: Raw `Word` from slot 4 map (encoded name)

**Usage**: Low-level testing, usually use `get_name_for_address()` instead

---

## Internal Helper Functions

### `word_to_masm_push_string()`
**Signature**: `fn word_to_masm_push_string(word: &Word) -> String`
**Purpose**: Convert Word to MASM push instruction string
**Visibility**: Private (not `pub`)

**Returns**: String like `"5.435459550305.0.0"` for `push.5.435459550305.0.0`

**Critical Note**:
```
push.a.b.c.d in MASM = Word([a,b,c,d]) in Rust (SAME ORDER)
Word elements are in order [e0, e1, e2, e3]
MASM push order matches Word array order directly
```

**Example**:
```rust
let word = Word::new([Felt::new(5), Felt::new(100), Felt::ZERO, Felt::ZERO]);
let push_str = word_to_masm_push_string(&word);
// Returns: "5.100.0.0"
// Used as: push.5.100.0.0
```

---

## Common Test Patterns

### Pattern 1: Basic Test Setup
```rust
#[tokio::test]
async fn test_something() -> Result<(), ClientError> {
    // Setup with deployed contract
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;

    // Create accounts
    let owner = helper.create_account("Owner").await?;
    let user = helper.create_account("User").await?;

    // Initialize registry
    helper.initialize_registry(&owner).await?;

    // ... test logic ...

    Ok(())
}
```

---

### Pattern 2: Name Registration & Verification
```rust
// Register name
helper.register_name_for_account(&user, "alice").await?;

// Verify forward mapping (Name→ID)
assert!(helper.is_name_registered("alice").await?);
let (prefix, suffix) = helper.get_account_for_name("alice").await?.unwrap();
assert_eq!(prefix, user.id().prefix().as_felt().as_int());

// Verify reverse mapping (ID→Name)
assert!(helper.has_name_for_address(&user).await?);
assert_eq!(
    helper.get_name_for_address(&user).await?,
    Some("alice".to_string())
);
```

---

### Pattern 3: Error Testing
```rust
// First registration should succeed
helper.register_name_for_account(&user1, "alice").await?;

// Duplicate should fail
let error = helper
    .register_name_for_account(&user2, "alice")
    .await
    .expect_err("duplicate registration should fail");

assert!(error.to_string().contains("name already registered"));
```

---

### Pattern 4: State Inspection
```rust
// Get contract state
let state = helper.get_contract_state().await?.unwrap();

// Check initialization
let (init_flag, _, _) = helper.get_initialization_state(&state);
assert_eq!(init_flag, 1);

// Check mappings
let (slot3_root, slot4_root) = helper.get_registry_mapping_state(&state);
assert!(slot3_root.is_some()); // Map has data
```

---

## Storage Slot Reference

| Slot | Content | Helper Method |
|------|---------|---------------|
| **0** | Initialized flag | `get_initialization_state()` |
| **1** | Owner (prefix, suffix) | `get_complete_contract_state()` |
| **2** | Payment token | `get_payment_token_state()` |
| **3** | Name→ID map | `get_account_for_name()`, `is_name_registered()` |
| **4** | ID→Name map | `get_name_for_address()`, `has_name_for_address()` |
| **5** | Price | (not exposed in helper) |

---

## Key Design Decisions

### Name Encoding
- **Format**: Length-prefixed with 7 chars per felt
- **Max Length**: 20 characters (enforced by contract)
- **Encoding**: Little-endian ASCII bytes
- **Rationale**: Simple POC approach, can be optimized later

### Network Handling
- **Testnet First**: Tries testnet, falls back to localhost
- **Sync Points**: After each transaction and state change
- **Delays**: Strategic sleeps for network propagation
- **Clean Start**: Deletes keystore/store on setup

### Test Isolation
- **Per-Test Cleanup**: Each test gets clean slate
- **Unique Accounts**: Fresh accounts per test
- **No Shared State**: Tests are independent

---

## Dependencies

**External Crates**:
- `miden_client`: Client SDK for Miden transactions
- `miden_objects`: Core Miden types (Felt, Word, etc.)
- `tokio`: Async runtime
- `rand`: RNG for keystore

**Internal**:
- `midenid_contracts::common`: Shared contract utilities
  - `instantiate_client()`
  - `create_basic_account()`
  - `create_public_immutable_contract()`
  - `create_library()`
  - `create_tx_script()`
  - `delete_keystore_and_store()`

---

## Example: Complete Test

```rust
#[tokio::test]
async fn test_complete_registration_flow() -> Result<(), ClientError> {
    // Setup
    let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
    let owner = helper.create_account("Owner").await?;
    let user = helper.create_account("User").await?;

    // Initialize
    helper.initialize_registry(&owner).await?;

    // Verify empty state
    assert!(!helper.is_name_registered("alice").await?);
    assert!(!helper.has_name_for_address(&user).await?);

    // Register name
    helper.register_name_for_account(&user, "alice").await?;

    // Verify forward mapping (slot 3)
    assert!(helper.is_name_registered("alice").await?);
    let (prefix, suffix) = helper.get_account_for_name("alice").await?.unwrap();
    assert_eq!(prefix, user.id().prefix().as_felt().as_int());
    assert_eq!(suffix, user.id().suffix().as_int());

    // Verify reverse mapping (slot 4)
    assert!(helper.has_name_for_address(&user).await?);
    assert_eq!(
        helper.get_name_for_address(&user).await?,
        Some("alice".to_string())
    );

    Ok(())
}
```

---
