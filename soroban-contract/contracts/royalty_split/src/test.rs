use crate::{RoyaltySplitContract, RoyaltyError, RoyaltyConfig, RoyaltyRecipient, DataKey};
use soroban_sdk::{
    testutils::{Address as AddressTestutils, Ledger},
    Address, Env, IntoVal,
};

fn create_env() -> Env {
    Env::default()
}

fn create_contract(e: &Env) -> Address {
    Address::generate(e)
}

#[test]
fn test_constructor_initializes_correctly() {
    let env = create_env();
    let admin = Address::generate(&env);
    let contract_id = create_contract(&env);

    env.as_contract(&contract_id, || {
        RoyaltySplitContract::__constructor(env.clone(), admin.clone());

        let config = RoyaltySplitContract::get_royalty_config(env.clone());
        assert!(config.is_some());

        let config = config.unwrap();
        assert!(!config.active);
        assert_eq!(config.total_percentage, 0);
        assert_eq!(config.max_recipients, 10);
        assert_eq!(config.recipients.len(), 0);
    });
}

#[test]
fn test_add_recipient_success() {
    let env = create_env();
    let admin = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let contract_id = create_contract(&env);

    env.as_contract(&contract_id, || {
        RoyaltySplitContract::__constructor(env.clone(), admin.clone());

        let result = RoyaltySplitContract::add_recipient(
            env.clone(),
            admin.clone(),
            recipient1.clone(),
            2500, // 25%
        );

        assert!(result.is_ok());

        let config = RoyaltySplitContract::get_royalty_config(env.clone()).unwrap();
        assert_eq!(config.recipients.len(), 1);
        assert_eq!(config.total_percentage, 2500);

        let recipient = config.recipients.get(0).unwrap();
        assert_eq!(recipient.recipient, recipient1);
        assert_eq!(recipient.percentage, 2500);
        assert!(recipient.active);
    });
}

#[test]
fn test_add_multiple_recipients() {
    let env = create_env();
    let admin = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipient3 = Address::generate(&env);
    let contract_id = create_contract(&env);

    env.as_contract(&contract_id, || {
        RoyaltySplitContract::__constructor(env.clone(), admin.clone());

        // Add first recipient - 25%
        RoyaltySplitContract::add_recipient(
            env.clone(),
            admin.clone(),
            recipient1.clone(),
            2500,
        )
        .unwrap();

        // Add second recipient - 35%
        RoyaltySplitContract::add_recipient(
            env.clone(),
            admin.clone(),
            recipient2.clone(),
            3500,
        )
        .unwrap();

        // Add third recipient - 15%
        RoyaltySplitContract::add_recipient(
            env.clone(),
            admin.clone(),
            recipient3.clone(),
            1500,
        )
        .unwrap();

        let config = RoyaltySplitContract::get_royalty_config(env.clone()).unwrap();
        assert_eq!(config.recipients.len(), 3);
        assert_eq!(config.total_percentage, 7500); // 75%
    });
}

#[test]
fn test_add_recipient_unauthorized() {
    let env = create_env();
    let admin = Address::generate(&env);
    let unauthorized = Address::generate(&env);
    let recipient = Address::generate(&env);
    let contract_id = create_contract(&env);

    env.as_contract(&contract_id, || {
        RoyaltySplitContract::__constructor(env.clone(), admin.clone());

        let result = RoyaltySplitContract::add_recipient(
            env.clone(),
            unauthorized.clone(),
            recipient.clone(),
            2500,
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), RoyaltyError::Unauthorized);
    });
}

#[test]
fn test_add_recipient_invalid_percentage() {
    let env = create_env();
    let admin = Address::generate(&env);
    let recipient = Address::generate(&env);
    let contract_id = create_contract(&env);

    env.as_contract(&contract_id, || {
        RoyaltySplitContract::__constructor(env.clone(), admin.clone());

        // Test 0%
        let result = RoyaltySplitContract::add_recipient(
            env.clone(),
            admin.clone(),
            recipient.clone(),
            0,
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), RoyaltyError::InvalidPercentage);

        // Test > 100%
        let result = RoyaltySplitContract::add_recipient(
            env.clone(),
            admin.clone(),
            recipient.clone(),
            10001,
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), RoyaltyError::InvalidPercentage);
    });
}

#[test]
fn test_add_recipient_duplicate() {
    let env = create_env();
    let admin = Address::generate(&env);
    let recipient = Address::generate(&env);
    let contract_id = create_contract(&env);

    env.as_contract(&contract_id, || {
        RoyaltySplitContract::__constructor(env.clone(), admin.clone());

        RoyaltySplitContract::add_recipient(
            env.clone(),
            admin.clone(),
            recipient.clone(),
            2500,
        )
        .unwrap();

        let result = RoyaltySplitContract::add_recipient(
            env.clone(),
            admin.clone(),
            recipient.clone(),
            3000,
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), RoyaltyError::RecipientAlreadyExists);
    });
}

#[test]
fn test_add_recipient_exceeds_total_percentage() {
    let env = create_env();
    let admin = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let contract_id = create_contract(&env);

    env.as_contract(&contract_id, || {
        RoyaltySplitContract::__constructor(env.clone(), admin.clone());

        // Add 80%
        RoyaltySplitContract::add_recipient(
            env.clone(),
            admin.clone(),
            recipient1.clone(),
            8000,
        )
        .unwrap();

        // Try to add 30% (total would be 110%)
        let result = RoyaltySplitContract::add_recipient(
            env.clone(),
            admin.clone(),
            recipient2.clone(),
            3000,
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), RoyaltyError::TotalPercentageExceeded);
    });
}

#[test]
fn test_update_recipient_percentage() {
    let env = create_env();
    let admin = Address::generate(&env);
    let recipient = Address::generate(&env);
    let contract_id = create_contract(&env);

    env.as_contract(&contract_id, || {
        RoyaltySplitContract::__constructor(env.clone(), admin.clone());

        RoyaltySplitContract::add_recipient(
            env.clone(),
            admin.clone(),
            recipient.clone(),
            2500,
        )
        .unwrap();

        // Update to 30%
        let result = RoyaltySplitContract::update_recipient_percentage(
            env.clone(),
            admin.clone(),
            recipient.clone(),
            3000,
        );

        assert!(result.is_ok());

        let config = RoyaltySplitContract::get_royalty_config(env.clone()).unwrap();
        let r = config.recipients.get(0).unwrap();
        assert_eq!(r.percentage, 3000);
        assert_eq!(config.total_percentage, 3000);
    });
}

#[test]
fn test_update_recipient_not_found() {
    let env = create_env();
    let admin = Address::generate(&env);
    let recipient = Address::generate(&env);
    let contract_id = create_contract(&env);

    env.as_contract(&contract_id, || {
        RoyaltySplitContract::__constructor(env.clone(), admin.clone());

        let result = RoyaltySplitContract::update_recipient_percentage(
            env.clone(),
            admin.clone(),
            recipient.clone(),
            3000,
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), RoyaltyError::RecipientNotFound);
    });
}

#[test]
fn test_remove_recipient() {
    let env = create_env();
    let admin = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let contract_id = create_contract(&env);

    env.as_contract(&contract_id, || {
        RoyaltySplitContract::__constructor(env.clone(), admin.clone());

        RoyaltySplitContract::add_recipient(
            env.clone(),
            admin.clone(),
            recipient1.clone(),
            2500,
        )
        .unwrap();

        RoyaltySplitContract::add_recipient(
            env.clone(),
            admin.clone(),
            recipient2.clone(),
            3500,
        )
        .unwrap();

        // Remove first recipient
        let result = RoyaltySplitContract::remove_recipient(
            env.clone(),
            admin.clone(),
            recipient1.clone(),
        );

        assert!(result.is_ok());

        let config = RoyaltySplitContract::get_royalty_config(env.clone()).unwrap();
        assert_eq!(config.total_percentage, 3500); // Only recipient2 remains

        let active_recipients = RoyaltySplitContract::get_active_recipients(env.clone());
        assert_eq!(active_recipients.len(), 1);
        assert_eq!(active_recipients.get(0).unwrap().recipient, recipient2);
    });
}

#[test]
fn test_calculate_royalties() {
    let env = create_env();
    let admin = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipient3 = Address::generate(&env);
    let contract_id = create_contract(&env);

    env.as_contract(&contract_id, || {
        RoyaltySplitContract::__constructor(env.clone(), admin.clone());

        RoyaltySplitContract::add_recipient(
            env.clone(),
            admin.clone(),
            recipient1.clone(),
            2500, // 25%
        )
        .unwrap();

        RoyaltySplitContract::add_recipient(
            env.clone(),
            admin.clone(),
            recipient2.clone(),
            3500, // 35%
        )
        .unwrap();

        RoyaltySplitContract::add_recipient(
            env.clone(),
            admin.clone(),
            recipient3.clone(),
            1500, // 15%
        )
        .unwrap();

        // Activate royalties
        RoyaltySplitContract::set_royalty_active(env.clone(), admin.clone(), true).unwrap();

        // Calculate royalties for 10000 token sale
        let royalties = RoyaltySplitContract::calculate_royalties(env.clone(), 10000);

        assert_eq!(royalties.len(), 3);

        // Check each recipient's share
        let r1 = royalties.get(0).unwrap();
        assert_eq!(r1.0, recipient1);
        assert_eq!(r1.1, 2500); // 25% of 10000

        let r2 = royalties.get(1).unwrap();
        assert_eq!(r2.0, recipient2);
        assert_eq!(r2.1, 3500); // 35% of 10000

        let r3 = royalties.get(2).unwrap();
        assert_eq!(r3.0, recipient3);
        assert_eq!(r3.1, 1500); // 15% of 10000
    });
}

#[test]
fn test_calculate_royalties_inactive() {
    let env = create_env();
    let admin = Address::generate(&env);
    let recipient = Address::generate(&env);
    let contract_id = create_contract(&env);

    env.as_contract(&contract_id, || {
        RoyaltySplitContract::__constructor(env.clone(), admin.clone());

        RoyaltySplitContract::add_recipient(
            env.clone(),
            admin.clone(),
            recipient.clone(),
            2500,
        )
        .unwrap();

        // Don't activate - should return empty
        let royalties = RoyaltySplitContract::calculate_royalties(env.clone(), 10000);
        assert_eq!(royalties.len(), 0);
    });
}

#[test]
fn test_set_royalty_active() {
    let env = create_env();
    let admin = Address::generate(&env);
    let contract_id = create_contract(&env);

    env.as_contract(&contract_id, || {
        RoyaltySplitContract::__constructor(env.clone(), admin.clone());

        // Activate
        RoyaltySplitContract::set_royalty_active(env.clone(), admin.clone(), true).unwrap();
        let config = RoyaltySplitContract::get_royalty_config(env.clone()).unwrap();
        assert!(config.active);

        // Deactivate
        RoyaltySplitContract::set_royalty_active(env.clone(), admin.clone(), false).unwrap();
        let config = RoyaltySplitContract::get_royalty_config(env.clone()).unwrap();
        assert!(!config.active);
    });
}

#[test]
fn test_set_max_recipients() {
    let env = create_env();
    let admin = Address::generate(&env);
    let contract_id = create_contract(&env);

    env.as_contract(&contract_id, || {
        RoyaltySplitContract::__constructor(env.clone(), admin.clone());

        // Update max recipients to 20
        let result = RoyaltySplitContract::set_max_recipients(env.clone(), admin.clone(), 20);
        assert!(result.is_ok());

        let config = RoyaltySplitContract::get_royalty_config(env.clone()).unwrap();
        assert_eq!(config.max_recipients, 20);
    });
}

#[test]
fn test_get_total_percentage() {
    let env = create_env();
    let admin = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let contract_id = create_contract(&env);

    env.as_contract(&contract_id, || {
        RoyaltySplitContract::__constructor(env.clone(), admin.clone());

        assert_eq!(RoyaltySplitContract::get_total_percentage(env.clone()), 0);

        RoyaltySplitContract::add_recipient(
            env.clone(),
            admin.clone(),
            recipient1.clone(),
            2500,
        )
        .unwrap();

        assert_eq!(RoyaltySplitContract::get_total_percentage(env.clone()), 2500);

        RoyaltySplitContract::add_recipient(
            env.clone(),
            admin.clone(),
            recipient2.clone(),
            3500,
        )
        .unwrap();

        assert_eq!(RoyaltySplitContract::get_total_percentage(env.clone()), 6000);
    });
}

#[test]
fn test_get_active_recipients() {
    let env = create_env();
    let admin = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipient3 = Address::generate(&env);
    let contract_id = create_contract(&env);

    env.as_contract(&contract_id, || {
        RoyaltySplitContract::__constructor(env.clone(), admin.clone());

        RoyaltySplitContract::add_recipient(
            env.clone(),
            admin.clone(),
            recipient1.clone(),
            2500,
        )
        .unwrap();

        RoyaltySplitContract::add_recipient(
            env.clone(),
            admin.clone(),
            recipient2.clone(),
            3500,
        )
        .unwrap();

        RoyaltySplitContract::add_recipient(
            env.clone(),
            admin.clone(),
            recipient3.clone(),
            1500,
        )
        .unwrap();

        // Remove recipient2
        RoyaltySplitContract::remove_recipient(env.clone(), admin.clone(), recipient2.clone())
            .unwrap();

        let active = RoyaltySplitContract::get_active_recipients(env.clone());
        assert_eq!(active.len(), 2);

        // Verify only active recipients are returned
        let r1 = active.get(0).unwrap();
        assert_eq!(r1.recipient, recipient1);
        assert!(r1.active);

        let r2 = active.get(1).unwrap();
        assert_eq!(r2.recipient, recipient3);
        assert!(r2.active);
    });
}

#[test]
fn test_version_and_upgrade_functions() {
    let env = create_env();
    let admin = Address::generate(&env);
    let contract_id = create_contract(&env);

    env.as_contract(&contract_id, || {
        RoyaltySplitContract::__constructor(env.clone(), admin.clone());

        // Version should be 0 initially
        let version = RoyaltySplitContract::version(env.clone());
        assert_eq!(version, 0);
    });
}

#[test]
fn test_royalty_calculation_with_different_amounts() {
    let env = create_env();
    let admin = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let contract_id = create_contract(&env);

    env.as_contract(&contract_id, || {
        RoyaltySplitContract::__constructor(env.clone(), admin.clone());

        RoyaltySplitContract::add_recipient(
            env.clone(),
            admin.clone(),
            recipient1.clone(),
            3000, // 30%
        )
        .unwrap();

        RoyaltySplitContract::add_recipient(
            env.clone(),
            admin.clone(),
            recipient2.clone(),
            2000, // 20%
        )
        .unwrap();

        RoyaltySplitContract::set_royalty_active(env.clone(), admin.clone(), true).unwrap();

        // Test with 50000 token sale
        let royalties = RoyaltySplitContract::calculate_royalties(env.clone(), 50000);

        assert_eq!(royalties.len(), 2);

        let r1 = royalties.get(0).unwrap();
        assert_eq!(r1.1, 15000); // 30% of 50000

        let r2 = royalties.get(1).unwrap();
        assert_eq!(r2.1, 10000); // 20% of 50000
    });
}

#[test]
fn test_max_recipients_limit() {
    let env = create_env();
    let admin = Address::generate(&env);
    let contract_id = create_contract(&env);

    env.as_contract(&contract_id, || {
        RoyaltySplitContract::__constructor(env.clone(), admin.clone());

        // Add 10 recipients (default max)
        for i in 0..10 {
            let recipient = Address::generate(&env);
            RoyaltySplitContract::add_recipient(
                env.clone(),
                admin.clone(),
                recipient,
                500, // 5% each
            )
            .unwrap();
        }

        // Try to add 11th recipient
        let recipient = Address::generate(&env);
        let result = RoyaltySplitContract::add_recipient(
            env.clone(),
            admin.clone(),
            recipient,
            500,
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), RoyaltyError::MaxRecipientsReached);
    });
}
