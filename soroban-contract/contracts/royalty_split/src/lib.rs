//! Royalty Split Contract for distributing secondary sale royalties
//! Supports multiple recipients with configurable percentages

#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, token, Address, BytesN, Env, Vec,
};

use upgradeable as upg;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum RoyaltyError {
    Unauthorized = 1,
    InvalidPercentage = 2,
    RecipientNotFound = 3,
    RecipientAlreadyExists = 4,
    TotalPercentageExceeded = 5,
    MaxRecipientsReached = 6,
    RoyaltyNotConfigured = 7,
    InvalidRecipient = 8,
}

/// Represents a royalty recipient with their payout percentage
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct RoyaltyRecipient {
    pub recipient: Address,
    pub percentage: u32, // Basis points (1/100th of a percent), max 10000 = 100%
    pub active: bool,
}

/// Configuration for royalty splits
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct RoyaltyConfig {
    pub recipients: Vec<RoyaltyRecipient>,
    pub total_percentage: u32,
    pub max_recipients: u32,
    pub active: bool,
}

/// Storage keys for the contract
#[contracttype]
pub enum DataKey {
    RoyaltyConfig,
    Admin,
    RecipientCount,
}

/// Constants
const MAX_TOTAL_PERCENTAGE: u32 = 10000; // 100% in basis points
const DEFAULT_MAX_RECIPIENTS: u32 = 10;

#[contract]
pub struct RoyaltySplitContract;

#[contractimpl]
impl RoyaltySplitContract {
    /// Initialize the contract with admin address
    pub fn __constructor(env: Env, admin: Address) {
        admin.require_auth();

        upg::set_admin(&env, &admin);
        upg::init_version(&env);

        env.storage().persistent().set(&DataKey::Admin, &admin);
        env.storage()
            .persistent()
            .set(&DataKey::RecipientCount, &0u32);

        // Initialize empty royalty config
        let empty_config = RoyaltyConfig {
            recipients: Vec::new(&env),
            total_percentage: 0,
            max_recipients: DEFAULT_MAX_RECIPIENTS,
            active: false,
        };

        env.storage()
            .persistent()
            .set(&DataKey::RoyaltyConfig, &empty_config);

        Self::extend_persistent_ttl(&env, &DataKey::Admin);
        Self::extend_persistent_ttl(&env, &DataKey::RecipientCount);
        Self::extend_persistent_ttl(&env, &DataKey::RoyaltyConfig);
    }

    /// Add a new royalty recipient
    pub fn add_recipient(
        env: Env,
        admin: Address,
        recipient: Address,
        percentage: u32,
    ) -> Result<(), RoyaltyError> {
        admin.require_auth();
        Self::require_admin(&env, &admin)?;

        if recipient == Address::from_account(&env) {
            return Err(RoyaltyError::InvalidRecipient);
        }

        if percentage == 0 || percentage > MAX_TOTAL_PERCENTAGE {
            return Err(RoyaltyError::InvalidPercentage);
        }

        let mut config: RoyaltyConfig = env
            .storage()
            .persistent()
            .get(&DataKey::RoyaltyConfig)
            .ok_or(RoyaltyError::RoyaltyNotConfigured)?;

        // Check if recipient already exists
        for i in 0..config.recipients.len() {
            let existing = config.recipients.get(i).unwrap();
            if existing.recipient == recipient && existing.active {
                return Err(RoyaltyError::RecipientAlreadyExists);
            }
        }

        // Check max recipients
        let recipient_count: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::RecipientCount)
            .unwrap_or(0);

        if recipient_count >= config.max_recipients {
            return Err(RoyaltyError::MaxRecipientsReached);
        }

        // Check total percentage
        let new_total = config
            .total_percentage
            .checked_add(percentage)
            .ok_or(RoyaltyError::TotalPercentageExceeded)?;

        if new_total > MAX_TOTAL_PERCENTAGE {
            return Err(RoyaltyError::TotalPercentageExceeded);
        }

        let new_recipient = RoyaltyRecipient {
            recipient: recipient.clone(),
            percentage,
            active: true,
        };

        config.recipients.push_back(new_recipient);
        config.total_percentage = new_total;

        env.storage()
            .persistent()
            .set(&DataKey::RoyaltyConfig, &config);
        env.storage()
            .persistent()
            .set(&DataKey::RecipientCount, &(recipient_count + 1));

        Self::extend_persistent_ttl(&env, &DataKey::RoyaltyConfig);
        Self::extend_persistent_ttl(&env, &DataKey::RecipientCount);

        env.events().publish(
            ("recipient_added",),
            (recipient, percentage, config.total_percentage),
        );

        Ok(())
    }

    /// Update an existing recipient's percentage
    pub fn update_recipient_percentage(
        env: Env,
        admin: Address,
        recipient: Address,
        new_percentage: u32,
    ) -> Result<(), RoyaltyError> {
        admin.require_auth();
        Self::require_admin(&env, &admin)?;

        if new_percentage == 0 || new_percentage > MAX_TOTAL_PERCENTAGE {
            return Err(RoyaltyError::InvalidPercentage);
        }

        let mut config: RoyaltyConfig = env
            .storage()
            .persistent()
            .get(&DataKey::RoyaltyConfig)
            .ok_or(RoyaltyError::RoyaltyNotConfigured)?;

        let mut found = false;
        let mut old_percentage: u32 = 0;

        for i in 0..config.recipients.len() {
            let mut existing = config.recipients.get(i).unwrap();
            if existing.recipient == recipient && existing.active {
                old_percentage = existing.percentage;
                existing.percentage = new_percentage;
                config.recipients.set(i, existing);
                found = true;
                break;
            }
        }

        if !found {
            return Err(RoyaltyError::RecipientNotFound);
        }

        // Recalculate total percentage
        let mut new_total: u32 = 0;
        for i in 0..config.recipients.len() {
            let r = config.recipients.get(i).unwrap();
            if r.active {
                new_total = new_total
                    .checked_add(r.percentage)
                    .ok_or(RoyaltyError::TotalPercentageExceeded)?;
            }
        }

        if new_total > MAX_TOTAL_PERCENTAGE {
            return Err(RoyaltyError::TotalPercentageExceeded);
        }

        config.total_percentage = new_total;

        env.storage()
            .persistent()
            .set(&DataKey::RoyaltyConfig, &config);
        Self::extend_persistent_ttl(&env, &DataKey::RoyaltyConfig);

        env.events().publish(
            ("recipient_updated",),
            (recipient, old_percentage, new_percentage, new_total),
        );

        Ok(())
    }

    /// Remove a royalty recipient
    pub fn remove_recipient(
        env: Env,
        admin: Address,
        recipient: Address,
    ) -> Result<(), RoyaltyError> {
        admin.require_auth();
        Self::require_admin(&env, &admin)?;

        let mut config: RoyaltyConfig = env
            .storage()
            .persistent()
            .get(&DataKey::RoyaltyConfig)
            .ok_or(RoyaltyError::RoyaltyNotConfigured)?;

        let mut found = false;
        let mut removed_percentage: u32 = 0;

        for i in 0..config.recipients.len() {
            let mut existing = config.recipients.get(i).unwrap();
            if existing.recipient == recipient && existing.active {
                removed_percentage = existing.percentage;
                existing.active = false;
                config.recipients.set(i, existing);
                found = true;
                break;
            }
        }

        if !found {
            return Err(RoyaltyError::RecipientNotFound);
        }

        // Recalculate total percentage
        let mut new_total: u32 = 0;
        for i in 0..config.recipients.len() {
            let r = config.recipients.get(i).unwrap();
            if r.active {
                new_total = new_total
                    .checked_add(r.percentage)
                    .ok_or(RoyaltyError::TotalPercentageExceeded)?;
            }
        }

        config.total_percentage = new_total;

        env.storage()
            .persistent()
            .set(&DataKey::RoyaltyConfig, &config);
        Self::extend_persistent_ttl(&env, &DataKey::RoyaltyConfig);

        env.events().publish(
            ("recipient_removed",),
            (recipient, removed_percentage, new_total),
        );

        Ok(())
    }

    /// Activate or deactivate the royalty system
    pub fn set_royalty_active(
        env: Env,
        admin: Address,
        active: bool,
    ) -> Result<(), RoyaltyError> {
        admin.require_auth();
        Self::require_admin(&env, &admin)?;

        let mut config: RoyaltyConfig = env
            .storage()
            .persistent()
            .get(&DataKey::RoyaltyConfig)
            .ok_or(RoyaltyError::RoyaltyNotConfigured)?;

        config.active = active;

        env.storage()
            .persistent()
            .set(&DataKey::RoyaltyConfig, &config);
        Self::extend_persistent_ttl(&env, &DataKey::RoyaltyConfig);

        env.events()
            .publish(("royalty_status_changed",), (active,));

        Ok(())
    }

    /// Update max recipients limit
    pub fn set_max_recipients(
        env: Env,
        admin: Address,
        max_recipients: u32,
    ) -> Result<(), RoyaltyError> {
        admin.require_auth();
        Self::require_admin(&env, &admin)?;

        if max_recipients == 0 || max_recipients > 50 {
            return Err(RoyaltyError::InvalidPercentage);
        }

        let mut config: RoyaltyConfig = env
            .storage()
            .persistent()
            .get(&DataKey::RoyaltyConfig)
            .ok_or(RoyaltyError::RoyaltyNotConfigured)?;

        let current_count: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::RecipientCount)
            .unwrap_or(0);

        if max_recipients < current_count {
            return Err(RoyaltyError::MaxRecipientsReached);
        }

        config.max_recipients = max_recipients;

        env.storage()
            .persistent()
            .set(&DataKey::RoyaltyConfig, &config);
        Self::extend_persistent_ttl(&env, &DataKey::RoyaltyConfig);

        env.events()
            .publish(("max_recipients_updated",), (max_recipients,));

        Ok(())
    }

    /// Calculate royalty amounts for each recipient based on sale price
    /// Returns Vec of (recipient, amount)
    pub fn calculate_royalties(env: Env, sale_price: i128) -> Vec<(Address, i128)> {
        let config: RoyaltyConfig = match env
            .storage()
            .persistent()
            .get(&DataKey::RoyaltyConfig)
        {
            Some(c) => c,
            None => return Vec::new(&env),
        };

        if !config.active || config.total_percentage == 0 {
            return Vec::new(&env);
        }

        let mut royalties = Vec::new(&env);

        for i in 0..config.recipients.len() {
            let recipient = config.recipients.get(i).unwrap();
            if recipient.active && recipient.percentage > 0 {
                // Calculate: (sale_price * percentage) / 10000
                let amount = (sale_price * recipient.percentage as i128) / MAX_TOTAL_PERCENTAGE as i128;
                if amount > 0 {
                    royalties.push_back((recipient.recipient.clone(), amount));
                }
            }
        }

        royalties
    }

    /// Distribute royalties to all recipients
    /// This should be called after a sale to distribute royalties
    pub fn distribute_royalties(
        env: Env,
        payment_token: Address,
        sale_price: i128,
    ) -> Result<(), RoyaltyError> {
        let config: RoyaltyConfig = env
            .storage()
            .persistent()
            .get(&DataKey::RoyaltyConfig)
            .ok_or(RoyaltyError::RoyaltyNotConfigured)?;

        if !config.active {
            return Ok(()); // Royalties not active, silently succeed
        }

        let token_client = token::Client::new(&env, &payment_token);

        for i in 0..config.recipients.len() {
            let recipient = config.recipients.get(i).unwrap();
            if recipient.active && recipient.percentage > 0 {
                let amount =
                    (sale_price * recipient.percentage as i128) / MAX_TOTAL_PERCENTAGE as i128;

                if amount > 0 {
                    // Transfer from the caller (marketplace contract) to recipient
                    token_client.transfer(
                        &env.current_contract_address(),
                        &recipient.recipient,
                        &amount,
                    );

                    env.events().publish(
                        ("royalty_paid",),
                        (recipient.recipient.clone(), amount, sale_price),
                    );
                }
            }
        }

        Ok(())
    }

    /// Get the current royalty configuration
    pub fn get_royalty_config(env: Env) -> Option<RoyaltyConfig> {
        env.storage()
            .persistent()
            .get(&DataKey::RoyaltyConfig)
    }

    /// Get all active recipients
    pub fn get_active_recipients(env: Env) -> Vec<RoyaltyRecipient> {
        let config: RoyaltyConfig = match env
            .storage()
            .persistent()
            .get(&DataKey::RoyaltyConfig)
        {
            Some(c) => c,
            None => return Vec::new(&env),
        };

        let mut active_recipients = Vec::new(&env);

        for i in 0..config.recipients.len() {
            let recipient = config.recipients.get(i).unwrap();
            if recipient.active {
                active_recipients.push_back(recipient);
            }
        }

        active_recipients
    }

    /// Get total royalty percentage
    pub fn get_total_percentage(env: Env) -> u32 {
        let config: RoyaltyConfig = match env
            .storage()
            .persistent()
            .get(&DataKey::RoyaltyConfig)
        {
            Some(c) => c,
            None => return 0,
        };

        config.total_percentage
    }

    // ── Upgrade / admin ──────────────────────────────────────────────────────

    pub fn schedule_upgrade(env: Env, new_wasm_hash: BytesN<32>) {
        upg::schedule_upgrade(&env, new_wasm_hash);
    }

    pub fn cancel_upgrade(env: Env) {
        upg::cancel_upgrade(&env);
    }

    pub fn commit_upgrade(env: Env) {
        upg::commit_upgrade(&env);
    }

    pub fn pause(env: Env) {
        upg::pause(&env);
    }

    pub fn unpause(env: Env) {
        upg::unpause(&env);
    }

    pub fn transfer_admin(env: Env, new_admin: Address) {
        upg::transfer_admin(&env, new_admin);
    }

    pub fn version(env: Env) -> u32 {
        upg::get_version(&env)
    }

    /// Helper to check if address is admin
    fn require_admin(env: &Env, admin: &Address) -> Result<(), RoyaltyError> {
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .ok_or(RoyaltyError::Unauthorized)?;

        if *admin != stored_admin {
            return Err(RoyaltyError::Unauthorized);
        }

        Ok(())
    }

    fn extend_persistent_ttl(env: &Env, key: &DataKey) {
        upg::extend_persistent_ttl(env, key);
    }
}

#[cfg(test)]
mod test;
