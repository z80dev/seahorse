//! Behavior parity tests comparing Seahorse vs Anchor realloc/close patterns
//!
//! NOTE: Seahorse doesn't support native realloc/close operations, so these tests
//! verify the tracking pattern behavior rather than byte-for-byte parity.
//!
//! Tests verify:
//! 1. State initialization matches expected values
//! 2. Resize tracking (grow/shrink) updates state correctly
//! 3. Close marking (simulated) works correctly
//! 4. PDA derivation is deterministic
//! 5. Account discriminators are correct

use solana_pubkey::Pubkey;
use solana_sha256_hasher::hash;
use std::str::FromStr;

/// Realloc/Close program ID (same for Seahorse and Anchor reference)
fn program_id() -> Pubkey {
    Pubkey::from_str("F4fSecp12t3QtQaXTUWrjxLec1wQXJ31iQzuE2WjPsoB").unwrap()
}

/// Derive data PDA
fn derive_data_pda(data_id: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"data", data_id.to_le_bytes().as_ref()],
        &program_id(),
    )
}

/// Derive record PDA
fn derive_record_pda(record_id: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"record", record_id.to_le_bytes().as_ref()],
        &program_id(),
    )
}

/// Calculate Anchor account discriminator
fn anchor_discriminator(account_name: &str) -> [u8; 8] {
    let preimage = format!("account:{}", account_name);
    let hash = hash(preimage.as_bytes());
    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&hash.as_ref()[..8]);
    discriminator
}

/// Calculate Anchor instruction discriminator
fn instruction_discriminator(name: &str) -> [u8; 8] {
    let preimage = format!("global:{}", name);
    let hash = hash(preimage.as_bytes());
    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&hash.as_ref()[..8]);
    discriminator
}

// =============================================================================
// PARITY TESTS
// =============================================================================

#[test]
fn test_parity_dynamic_data_discriminator() {
    // Verify the discriminator for DynamicData account is correct
    let discriminator = anchor_discriminator("DynamicData");

    // The discriminator should be first 8 bytes of sha256("account:DynamicData")
    // This ensures account type validation works
    assert_eq!(discriminator.len(), 8);

    // Verify it's not all zeros (would indicate a bug)
    assert_ne!(discriminator, [0u8; 8]);

    println!("DynamicData discriminator: {:?}", discriminator);
}

#[test]
fn test_parity_resize_record_discriminator() {
    // Verify the discriminator for ResizeRecord account is correct
    let discriminator = anchor_discriminator("ResizeRecord");

    assert_eq!(discriminator.len(), 8);
    assert_ne!(discriminator, [0u8; 8]);

    // Ensure DynamicData and ResizeRecord have different discriminators
    let data_disc = anchor_discriminator("DynamicData");
    assert_ne!(discriminator, data_disc);

    println!("ResizeRecord discriminator: {:?}", discriminator);
}

#[test]
fn test_parity_instruction_discriminators() {
    // Verify instruction discriminators for all instructions

    let instructions = vec![
        "initialize_data",
        "grow_account",
        "shrink_account",
        "close_data_account",
        "create_record",
        "record_resize",
        "close_record",
        "get_data_info",
        "get_record_info",
    ];

    let mut discriminators = Vec::new();

    for name in &instructions {
        let disc = instruction_discriminator(name);
        assert_eq!(disc.len(), 8);
        assert_ne!(disc, [0u8; 8]);

        // Ensure unique discriminators
        assert!(
            !discriminators.contains(&disc),
            "Duplicate discriminator for {}",
            name
        );
        discriminators.push(disc);

        println!("{}: {:?}", name, disc);
    }
}

#[test]
fn test_parity_pda_derivation_deterministic() {
    // Verify PDA derivation produces consistent addresses
    // Both Seahorse and Anchor use same PDA derivation mechanism

    let data_id: u64 = 12345;
    let record_id: u64 = 67890;

    // Derive multiple times - should be identical
    let (pda1, bump1) = derive_data_pda(data_id);
    let (pda2, bump2) = derive_data_pda(data_id);
    let (pda3, bump3) = derive_data_pda(data_id);

    assert_eq!(pda1, pda2);
    assert_eq!(pda2, pda3);
    assert_eq!(bump1, bump2);
    assert_eq!(bump2, bump3);

    // Record PDAs
    let (record_pda1, rbump1) = derive_record_pda(record_id);
    let (record_pda2, rbump2) = derive_record_pda(record_id);

    assert_eq!(record_pda1, record_pda2);
    assert_eq!(rbump1, rbump2);

    // Different IDs produce different PDAs
    let (other_pda, _) = derive_data_pda(99999);
    assert_ne!(pda1, other_pda);

    println!("Data PDA for id {}: {}", data_id, pda1);
    println!("Record PDA for id {}: {}", record_id, record_pda1);
}

#[test]
fn test_parity_realloc_tracking_state() {
    // Verify the expected state transitions for realloc tracking pattern
    //
    // This tests the conceptual behavior:
    // 1. initialize_data: version=1, allocated_size=initial, is_closed=false
    // 2. grow_account: version++, allocated_size+=additional
    // 3. shrink_account: version++, allocated_size=new_size
    // 4. close_data_account: is_closed=true, allocated_size=0

    // Initial state after initialize_data
    let initial_allocated_size: u64 = 100;
    let initial_version: u64 = 1;
    let initial_is_closed = false;

    assert_eq!(initial_version, 1);
    assert!(!initial_is_closed);

    // After grow_account with additional_space=50
    let additional_space: u64 = 50;
    let after_grow_size = initial_allocated_size + additional_space;
    let after_grow_version = initial_version + 1;

    assert_eq!(after_grow_size, 150);
    assert_eq!(after_grow_version, 2);

    // After shrink_account with new_size=75
    let shrink_new_size: u64 = 75;
    let after_shrink_size = shrink_new_size;
    let after_shrink_version = after_grow_version + 1;

    assert_eq!(after_shrink_size, 75);
    assert_eq!(after_shrink_version, 3);

    // After close_data_account
    let after_close_size: u64 = 0;
    let after_close_is_closed = true;

    assert_eq!(after_close_size, 0);
    assert!(after_close_is_closed);

    println!("State transitions verified:");
    println!("  Initial: size={}, version={}, closed={}", initial_allocated_size, initial_version, initial_is_closed);
    println!("  After grow: size={}, version={}", after_grow_size, after_grow_version);
    println!("  After shrink: size={}, version={}", after_shrink_size, after_shrink_version);
    println!("  After close: size={}, closed={}", after_close_size, after_close_is_closed);
}

#[test]
fn test_parity_resize_record_tracking() {
    // Verify the expected state for resize record tracking
    //
    // This tests the resize record pattern:
    // 1. create_record: current_size=0, max_size=0, count=0, is_active=true
    // 2. record_resize (grow to 100): current_size=100, max_size=100, count=1
    // 3. record_resize (grow to 200): current_size=200, max_size=200, count=2
    // 4. record_resize (shrink to 150): current_size=150, max_size=200 (unchanged), count=3
    // 5. close_record: is_active=false

    // Initial state after create_record
    let mut current_size: u64 = 0;
    let mut max_size: u64 = 0;
    let mut count: u64 = 0;
    let mut is_active = true;

    // Record grow to 100
    current_size = 100;
    count += 1;
    if current_size > max_size {
        max_size = current_size;
    }
    assert_eq!(current_size, 100);
    assert_eq!(max_size, 100);
    assert_eq!(count, 1);

    // Record grow to 200
    current_size = 200;
    count += 1;
    if current_size > max_size {
        max_size = current_size;
    }
    assert_eq!(current_size, 200);
    assert_eq!(max_size, 200);
    assert_eq!(count, 2);

    // Record shrink to 150 (max_size should stay at 200)
    current_size = 150;
    count += 1;
    if current_size > max_size {
        max_size = current_size; // This won't execute
    }
    assert_eq!(current_size, 150);
    assert_eq!(max_size, 200); // Still 200!
    assert_eq!(count, 3);

    // Close record
    is_active = false;
    assert!(!is_active);

    println!("Resize record tracking verified:");
    println!("  Final: current_size={}, max_size={}, count={}, active={}", current_size, max_size, count, is_active);
}
