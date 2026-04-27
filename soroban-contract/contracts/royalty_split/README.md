# Royalty Split Contract

A comprehensive royalty distribution system for the TokenBound ticketing platform that enables automatic splitting of secondary sale royalties among multiple recipients with configurable percentages.

## Overview

The Royalty Split Contract provides a flexible and secure mechanism for distributing royalties from secondary ticket sales. It supports:

- **Multiple Recipients**: Configure up to 10 (customizable) royalty recipients
- **Flexible Percentages**: Each recipient receives a configurable percentage (in basis points)
- **Updatable Configuration**: Admin can add, update, or remove recipients at any time
- **Automatic Distribution**: Integrates with marketplace for automatic royalty distribution on sales
- **Basis Points Precision**: Uses basis points (1/100th of a percent) for accurate calculations

## Architecture

### Data Structures

#### RoyaltyRecipient
```rust
pub struct RoyaltyRecipient {
    pub recipient: Address,    // Recipient's blockchain address
    pub percentage: u32,       // Percentage in basis points (100 = 1%, 10000 = 100%)
    pub active: bool,          // Whether this recipient is active
}
```

#### RoyaltyConfig
```rust
pub struct RoyaltyConfig {
    pub recipients: Vec<RoyaltyRecipient>,  // List of all recipients
    pub total_percentage: u32,              // Sum of all active percentages
    pub max_recipients: u32,                // Maximum allowed recipients
    pub active: bool,                       // Whether royalty system is active
}
```

### Storage Keys

- `RoyaltyConfig`: Main configuration storing all recipients and settings
- `Admin`: Contract administrator address
- `RecipientCount`: Current number of active recipients

## Contract Functions

### Initialization

#### `__constructor(env, admin)`
Initializes the royalty split contract.

**Parameters:**
- `admin`: Address of the contract administrator

**Example:**
```rust
RoyaltySplitContract::__constructor(env, admin_address);
```

### Recipient Management

#### `add_recipient(env, admin, recipient, percentage)`
Adds a new royalty recipient.

**Parameters:**
- `admin`: Admin address (must be authorized)
- `recipient`: Address of the royalty recipient
- `percentage`: Percentage in basis points (1-10000)

**Returns:** `Result<(), RoyaltyError>`

**Example:**
```rust
// Add recipient with 25% royalty share
RoyaltySplitContract::add_recipient(env, admin, recipient_addr, 2500);
```

**Errors:**
- `Unauthorized`: Caller is not admin
- `InvalidPercentage`: Percentage is 0 or > 10000
- `RecipientAlreadyExists`: Recipient already in the list
- `MaxRecipientsReached`: Cannot add more recipients
- `TotalPercentageExceeded`: Total would exceed 100%

#### `update_recipient_percentage(env, admin, recipient, new_percentage)`
Updates an existing recipient's percentage.

**Parameters:**
- `admin`: Admin address (must be authorized)
- `recipient`: Address of the recipient to update
- `new_percentage`: New percentage in basis points

**Returns:** `Result<(), RoyaltyError>`

**Example:**
```rust
// Update recipient to 30%
RoyaltySplitContract::update_recipient_percentage(env, admin, recipient_addr, 3000);
```

#### `remove_recipient(env, admin, recipient)`
Removes a royalty recipient (marks as inactive).

**Parameters:**
- `admin`: Admin address (must be authorized)
- `recipient`: Address of the recipient to remove

**Returns:** `Result<(), RoyaltyError>`

**Example:**
```rust
RoyaltySplitContract::remove_recipient(env, admin, recipient_addr);
```

### Configuration

#### `set_royalty_active(env, admin, active)`
Activates or deactivates the royalty system.

**Parameters:**
- `admin`: Admin address (must be authorized)
- `active`: Boolean to activate/deactivate

**Example:**
```rust
// Activate royalty distribution
RoyaltySplitContract::set_royalty_active(env, admin, true);
```

#### `set_max_recipients(env, admin, max_recipients)`
Updates the maximum number of allowed recipients.

**Parameters:**
- `admin`: Admin address (must be authorized)
- `max_recipients`: New maximum (1-50)

**Example:**
```rust
RoyaltySplitContract::set_max_recipients(env, admin, 20);
```

### Royalty Calculation & Distribution

#### `calculate_royalties(env, sale_price)`
Calculates royalty amounts for each recipient based on sale price.

**Parameters:**
- `sale_price`: Total sale price in tokens

**Returns:** `Vec<(Address, i128)>` - Vector of (recipient, amount) pairs

**Example:**
```rust
let royalties = RoyaltySplitContract::calculate_royalties(env, 10000);
// Returns: [(recipient1, 2500), (recipient2, 3500), ...]
```

**Calculation:**
```rust
amount = (sale_price * percentage) / 10000
```

#### `distribute_royalties(env, payment_token, sale_price)`
Distributes royalties to all active recipients.

**Parameters:**
- `payment_token`: Token contract address for payment
- `sale_price`: Total sale price

**Returns:** `Result<(), RoyaltyError>`

**Example:**
```rust
RoyaltySplitContract::distribute_royalties(env, token_address, 10000);
```

### Query Functions

#### `get_royalty_config(env)`
Returns the complete royalty configuration.

**Returns:** `Option<RoyaltyConfig>`

#### `get_active_recipients(env)`
Returns all active royalty recipients.

**Returns:** `Vec<RoyaltyRecipient>`

#### `get_total_percentage(env)`
Returns the total royalty percentage across all active recipients.

**Returns:** `u32` - Total percentage in basis points

## Marketplace Integration

The marketplace contract has been updated to support automatic royalty distribution:

### New Marketplace Functions

#### `set_royalty_contract(env, admin, royalty_contract)`
Configures the royalty split contract address in the marketplace.

**Example:**
```rust
MarketplaceContract::set_royalty_contract(env, admin, royalty_contract_address);
```

#### `set_marketplace_fee(env, admin, fee_percentage)`
Sets the marketplace fee percentage (in basis points).

**Example:**
```rust
// Set 2.5% marketplace fee
MarketplaceContract::set_marketplace_fee(env, admin, 250);
```

#### `get_marketplace_fee(env)`
Returns the current marketplace fee percentage.

**Returns:** `u32`

#### `get_royalty_contract(env)`
Returns the configured royalty contract address.

**Returns:** `Option<Address>`

### Purchase Flow with Royalties

When a ticket is purchased through the marketplace:

1. **Calculate Fees:**
   - Marketplace fee (e.g., 2.5%)
   - Total royalties (e.g., 5%)
   - Seller receives: `price - marketplace_fee - royalties`

2. **Distribute Payments:**
   - Transfer total amount from buyer to marketplace
   - Transfer marketplace fee to admin
   - Transfer royalties to royalty contract for distribution
   - Transfer remaining amount to seller

3. **Example Breakdown:**
   ```
   Sale Price: 10,000 tokens
   Marketplace Fee: 2.5% = 250 tokens
   Royalties: 5% = 500 tokens
   Seller Receives: 9,250 tokens
   
   Royalty Distribution (if 2 recipients):
   - Recipient 1 (60% of royalties): 300 tokens
   - Recipient 2 (40% of royalties): 200 tokens
   ```

## Usage Examples

### Complete Setup Flow

```rust
// 1. Deploy and initialize royalty contract
RoyaltySplitContract::__constructor(env, admin);

// 2. Add royalty recipients
RoyaltySplitContract::add_recipient(env, admin, creator_address, 3000);    // 30%
RoyaltySplitContract::add_recipient(env, admin, platform_address, 1500);   // 15%
RoyaltySplitContract::add_recipient(env, admin, charity_address, 500);     // 5%

// 3. Activate royalty system
RoyaltySplitContract::set_royalty_active(env, admin, true);

// 4. Configure marketplace to use royalty contract
MarketplaceContract::set_royalty_contract(env, admin, royalty_contract);

// 5. Set marketplace fee
MarketplaceContract::set_marketplace_fee(env, admin, 250); // 2.5%
```

### Query Royalty Information

```rust
// Get total royalty percentage
let total_pct = RoyaltySplitContract::get_total_percentage(env);
// Returns: 5000 (50%)

// Get all active recipients
let recipients = RoyaltySplitContract::get_active_recipients(env);
for r in recipients {
    println!("Recipient: {}, Percentage: {}%", r.recipient, r.percentage / 100);
}

// Calculate royalties for a sale
let royalties = RoyaltySplitContract::calculate_royalties(env, 20000);
for (recipient, amount) in royalties {
    println!("{} receives {} tokens", recipient, amount);
}
```

## Error Handling

### RoyaltyError Enum

```rust
pub enum RoyaltyError {
    Unauthorized = 1,              // Caller is not authorized
    InvalidPercentage = 2,         // Percentage is invalid
    RecipientNotFound = 3,         // Recipient not in the list
    RecipientAlreadyExists = 4,    // Duplicate recipient
    TotalPercentageExceeded = 5,   // Total > 100%
    MaxRecipientsReached = 6,      // Cannot add more recipients
    RoyaltyNotConfigured = 7,      // Royalty system not initialized
    InvalidRecipient = 8,          // Invalid recipient address
}
```

## Security Considerations

1. **Admin Authorization**: All configuration changes require admin authentication
2. **Percentage Validation**: Ensures total percentage never exceeds 100%
3. **Overflow Protection**: Uses checked arithmetic for all calculations
4. **Recipient Limits**: Prevents excessive number of recipients
5. **Active Status**: Recipients can be deactivated without removal

## Testing

The contract includes comprehensive tests covering:

- ✅ Constructor initialization
- ✅ Adding recipients
- ✅ Updating recipient percentages
- ✅ Removing recipients
- ✅ Percentage validation
- ✅ Duplicate prevention
- ✅ Maximum recipient limits
- ✅ Royalty calculation accuracy
- ✅ Active/inactive status
- ✅ Authorization checks
- ✅ Edge cases and error conditions

Run tests with:
```bash
cd soroban-contract
cargo test -p royalty_split
```

## Deployment

### Build the Contract

```bash
cd soroban-contract
cargo build --target wasm32-unknown-unknown --release -p royalty_split
```

### Deploy to Soroban

```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/royalty_split.wasm \
  --source <source_account> \
  --network <network>
```

### Initialize Contract

```bash
soroban contract invoke \
  --id <contract_id> \
  --source <source_account> \
  --network <network> \
  -- __constructor \
  --admin <admin_address>
```

## Best Practices

1. **Start with Lower Percentages**: Begin with conservative royalty percentages
2. **Regular Audits**: Periodically review and update recipient list
3. **Test Thoroughly**: Test all configurations before mainnet deployment
4. **Monitor Distribution**: Verify royalty distributions match expectations
5. **Document Recipients**: Keep clear records of who receives royalties and why
6. **Use Basis Points**: Always think in basis points for precision (100 = 1%)

## Future Enhancements

- [ ] Support for different royalty rates per event/tier
- [ ] Time-based royalty adjustments
- [ ] Automated royalty claim mechanism
- [ ] Royalty history tracking
- [ ] Multi-token support
- [ ] Governance-based configuration updates

## License

This contract is part of the TokenBound ticketing platform.
