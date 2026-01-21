//! LiteSVM integration tests for the Seahorse NFT minting program
//!
//! These tests verify the NFT configuration management functionality.
//! Note: Seahorse doesn't have native Metaplex Token Metadata support, so these tests
//! focus on the NFT config tracking functionality (similar to the Token-2022 pattern).
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile nft_mint.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::str::FromStr;

/// NFT minting program ID (from declare_id!)
fn nft_mint_program_id() -> Pubkey {
    Pubkey::from_str("NFTm1nt111111111111111111111111111111111111").unwrap()
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the nft_mint program into LiteSVM
fn load_nft_mint_program() -> litesvm::LiteSVM {
    let program_id = nft_mint_program_id();
    let program_bytes = std::fs::read("../../target/deploy/nft_mint.so")
        .expect("Failed to read nft_mint.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);
    svm
}

/// Derive nft_config PDA
fn derive_nft_config_pda(nft_id: u64) -> (Pubkey, u8) {
    find_pda(
        &[b"nft_config", &nft_id.to_le_bytes()],
        &nft_mint_program_id(),
    )
}

/// Read NftConfig fields from account data
/// Layout: discriminator (8) + authority (32) + nft_id (8) + mint (32) +
///         name (4+len) + symbol (4+len) + uri (4+len) + is_minted (1) + bump (1)
fn read_nft_config(data: &[u8]) -> (Pubkey, u64, Pubkey, String, String, String, bool, u8) {
    let authority = Pubkey::new_from_array(data[8..40].try_into().unwrap());
    let nft_id = u64::from_le_bytes(data[40..48].try_into().unwrap());
    let mint = Pubkey::new_from_array(data[48..80].try_into().unwrap());

    // Borsh strings: 4-byte length prefix (u32 LE) + bytes
    let name_len = u32::from_le_bytes(data[80..84].try_into().unwrap()) as usize;
    let name = String::from_utf8(data[84..84 + name_len].to_vec()).unwrap_or_default();

    let symbol_offset = 84 + name_len;
    let symbol_len = u32::from_le_bytes(data[symbol_offset..symbol_offset + 4].try_into().unwrap()) as usize;
    let symbol = String::from_utf8(data[symbol_offset + 4..symbol_offset + 4 + symbol_len].to_vec()).unwrap_or_default();

    let uri_offset = symbol_offset + 4 + symbol_len;
    let uri_len = u32::from_le_bytes(data[uri_offset..uri_offset + 4].try_into().unwrap()) as usize;
    let uri = String::from_utf8(data[uri_offset + 4..uri_offset + 4 + uri_len].to_vec()).unwrap_or_default();

    let is_minted_offset = uri_offset + 4 + uri_len;
    let is_minted = data[is_minted_offset] != 0;

    let bump_offset = is_minted_offset + 1;
    let bump = data[bump_offset];

    (authority, nft_id, mint, name, symbol, uri, is_minted, bump)
}

/// Serialize a string as Borsh (4-byte length prefix + bytes)
fn serialize_string(s: &str) -> Vec<u8> {
    let mut result = Vec::new();
    result.extend_from_slice(&(s.len() as u32).to_le_bytes());
    result.extend_from_slice(s.as_bytes());
    result
}

// =============================================================================
// CREATE NFT CONFIG TESTS
// =============================================================================

#[test]
fn test_create_nft_config() {
    let mut svm = load_nft_mint_program();
    let program_id = nft_mint_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let nft_id: u64 = 1;
    let (nft_config_pda, _) = derive_nft_config_pda(nft_id);

    let name = "Test NFT";
    let symbol = "TNFT";
    let uri = "https://example.com/metadata.json";

    // Build instruction data: nft_id (u64) + name (string) + symbol (string) + uri (string)
    let mut ix_data = Vec::new();
    ix_data.extend_from_slice(&nft_id.to_le_bytes());
    ix_data.extend_from_slice(&serialize_string(name));
    ix_data.extend_from_slice(&serialize_string(symbol));
    ix_data.extend_from_slice(&serialize_string(uri));

    let ix = anchor_instruction(
        program_id,
        "create_nft_config",
        &ix_data,
        vec![
            signer_meta(authority.pubkey()),           // authority
            writable_meta(nft_config_pda),             // nft_config
            readonly_meta(solana_sdk_ids::sysvar::rent::id()), // rent
            readonly_meta(system_program::id()),       // system_program
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(
        result.is_ok(),
        "Create NFT config should succeed: {:?}",
        result
    );

    // Verify nft_config was created
    let nft_config_account = svm
        .get_account(&nft_config_pda)
        .expect("NftConfig should exist");
    assert_eq!(
        nft_config_account.owner, program_id,
        "NftConfig should be owned by program"
    );

    let (
        stored_authority,
        stored_nft_id,
        stored_mint,
        stored_name,
        stored_symbol,
        stored_uri,
        is_minted,
        _bump,
    ) = read_nft_config(&nft_config_account.data);

    assert_eq!(stored_authority, authority.pubkey(), "Authority should match");
    assert_eq!(stored_nft_id, nft_id, "NFT ID should match");
    // Mint is placeholder (set to authority until mark_as_minted)
    assert_eq!(stored_mint, authority.pubkey(), "Mint should be placeholder");
    assert_eq!(stored_name, name, "Name should match");
    assert_eq!(stored_symbol, symbol, "Symbol should match");
    assert_eq!(stored_uri, uri, "URI should match");
    assert!(!is_minted, "Should not be minted initially");
}

#[test]
fn test_create_nft_config_different_ids() {
    let mut svm = load_nft_mint_program();
    let program_id = nft_mint_program_id();

    let authority = funded_keypair_10_sol(&mut svm);

    // Create first NFT config
    let nft_id_1: u64 = 100;
    let (nft_config_pda_1, _) = derive_nft_config_pda(nft_id_1);

    let mut ix_data_1 = Vec::new();
    ix_data_1.extend_from_slice(&nft_id_1.to_le_bytes());
    ix_data_1.extend_from_slice(&serialize_string("First NFT"));
    ix_data_1.extend_from_slice(&serialize_string("NFT1"));
    ix_data_1.extend_from_slice(&serialize_string("https://example.com/1.json"));

    let ix_1 = anchor_instruction(
        program_id,
        "create_nft_config",
        &ix_data_1,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(nft_config_pda_1),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, ix_1, &authority, &[&authority]).unwrap();

    // Create second NFT config with different ID
    svm.expire_blockhash();
    let nft_id_2: u64 = 200;
    let (nft_config_pda_2, _) = derive_nft_config_pda(nft_id_2);

    let mut ix_data_2 = Vec::new();
    ix_data_2.extend_from_slice(&nft_id_2.to_le_bytes());
    ix_data_2.extend_from_slice(&serialize_string("Second NFT"));
    ix_data_2.extend_from_slice(&serialize_string("NFT2"));
    ix_data_2.extend_from_slice(&serialize_string("https://example.com/2.json"));

    let ix_2 = anchor_instruction(
        program_id,
        "create_nft_config",
        &ix_data_2,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(nft_config_pda_2),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix_2, &authority, &[&authority]);
    assert!(result.is_ok(), "Should create second NFT config");

    // Verify both exist
    assert!(svm.get_account(&nft_config_pda_1).is_some(), "First NFT config should exist");
    assert!(svm.get_account(&nft_config_pda_2).is_some(), "Second NFT config should exist");
    assert_ne!(nft_config_pda_1, nft_config_pda_2, "PDAs should be different");
}

#[test]
fn test_create_nft_config_name_too_long_fails() {
    let mut svm = load_nft_mint_program();
    let program_id = nft_mint_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let nft_id: u64 = 1;
    let (nft_config_pda, _) = derive_nft_config_pda(nft_id);

    // Name exceeds 32 characters
    let long_name = "This name is way too long for an NFT name!";
    assert!(long_name.len() > 32);

    let mut ix_data = Vec::new();
    ix_data.extend_from_slice(&nft_id.to_le_bytes());
    ix_data.extend_from_slice(&serialize_string(long_name));
    ix_data.extend_from_slice(&serialize_string("SYM"));
    ix_data.extend_from_slice(&serialize_string("https://example.com"));

    let ix = anchor_instruction(
        program_id,
        "create_nft_config",
        &ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(nft_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_err(), "Name too long should fail");
}

#[test]
fn test_create_nft_config_symbol_too_long_fails() {
    let mut svm = load_nft_mint_program();
    let program_id = nft_mint_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let nft_id: u64 = 1;
    let (nft_config_pda, _) = derive_nft_config_pda(nft_id);

    // Symbol exceeds 10 characters
    let long_symbol = "VERYLONGSYM";
    assert!(long_symbol.len() > 10);

    let mut ix_data = Vec::new();
    ix_data.extend_from_slice(&nft_id.to_le_bytes());
    ix_data.extend_from_slice(&serialize_string("Test NFT"));
    ix_data.extend_from_slice(&serialize_string(long_symbol));
    ix_data.extend_from_slice(&serialize_string("https://example.com"));

    let ix = anchor_instruction(
        program_id,
        "create_nft_config",
        &ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(nft_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_err(), "Symbol too long should fail");
}

// =============================================================================
// UPDATE NFT CONFIG TESTS
// =============================================================================

#[test]
fn test_update_nft_config() {
    let mut svm = load_nft_mint_program();
    let program_id = nft_mint_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let nft_id: u64 = 1;
    let (nft_config_pda, _) = derive_nft_config_pda(nft_id);

    // Create NFT config
    let mut create_ix_data = Vec::new();
    create_ix_data.extend_from_slice(&nft_id.to_le_bytes());
    create_ix_data.extend_from_slice(&serialize_string("Original Name"));
    create_ix_data.extend_from_slice(&serialize_string("ORIG"));
    create_ix_data.extend_from_slice(&serialize_string("https://old.example.com"));

    let create_ix = anchor_instruction(
        program_id,
        "create_nft_config",
        &create_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(nft_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Update NFT config
    svm.expire_blockhash();
    let new_name = "Updated Name";
    let new_symbol = "UPDT";
    let new_uri = "https://new.example.com";

    let mut update_ix_data = Vec::new();
    update_ix_data.extend_from_slice(&serialize_string(new_name));
    update_ix_data.extend_from_slice(&serialize_string(new_symbol));
    update_ix_data.extend_from_slice(&serialize_string(new_uri));

    let update_ix = anchor_instruction(
        program_id,
        "update_nft_config",
        &update_ix_data,
        vec![
            signer_meta(authority.pubkey()), // authority
            writable_meta(nft_config_pda),   // nft_config
        ],
    );

    let result = execute_tx(&mut svm, update_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "Update should succeed: {:?}", result);

    // Verify updated values
    let nft_config_account = svm.get_account(&nft_config_pda).unwrap();
    let (_, _, _, stored_name, stored_symbol, stored_uri, _, _) = read_nft_config(&nft_config_account.data);

    assert_eq!(stored_name, new_name, "Name should be updated");
    assert_eq!(stored_symbol, new_symbol, "Symbol should be updated");
    assert_eq!(stored_uri, new_uri, "URI should be updated");
}

#[test]
fn test_update_nft_config_unauthorized_fails() {
    let mut svm = load_nft_mint_program();
    let program_id = nft_mint_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let attacker = funded_keypair_10_sol(&mut svm);
    let nft_id: u64 = 1;
    let (nft_config_pda, _) = derive_nft_config_pda(nft_id);

    // Create NFT config by authority
    let mut create_ix_data = Vec::new();
    create_ix_data.extend_from_slice(&nft_id.to_le_bytes());
    create_ix_data.extend_from_slice(&serialize_string("My NFT"));
    create_ix_data.extend_from_slice(&serialize_string("MNFT"));
    create_ix_data.extend_from_slice(&serialize_string("https://example.com"));

    let create_ix = anchor_instruction(
        program_id,
        "create_nft_config",
        &create_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(nft_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Attacker tries to update
    svm.expire_blockhash();
    let mut update_ix_data = Vec::new();
    update_ix_data.extend_from_slice(&serialize_string("Hacked Name"));
    update_ix_data.extend_from_slice(&serialize_string("HACK"));
    update_ix_data.extend_from_slice(&serialize_string("https://evil.com"));

    let update_ix = anchor_instruction(
        program_id,
        "update_nft_config",
        &update_ix_data,
        vec![
            signer_meta(attacker.pubkey()), // attacker signs
            writable_meta(nft_config_pda),  // but nft_config.authority != attacker
        ],
    );

    let result = execute_tx(&mut svm, update_ix, &attacker, &[&attacker]);
    assert!(result.is_err(), "Unauthorized update should fail");
}

#[test]
fn test_update_nft_config_after_minting_fails() {
    let mut svm = load_nft_mint_program();
    let program_id = nft_mint_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let nft_id: u64 = 1;
    let (nft_config_pda, _) = derive_nft_config_pda(nft_id);

    // Create NFT config
    let mut create_ix_data = Vec::new();
    create_ix_data.extend_from_slice(&nft_id.to_le_bytes());
    create_ix_data.extend_from_slice(&serialize_string("Test NFT"));
    create_ix_data.extend_from_slice(&serialize_string("TNFT"));
    create_ix_data.extend_from_slice(&serialize_string("https://example.com"));

    let create_ix = anchor_instruction(
        program_id,
        "create_nft_config",
        &create_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(nft_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Create a mock mint and mark as minted
    svm.expire_blockhash();
    let mock_mint = Keypair::new();
    let mint_account = create_mint_account(&authority.pubkey(), 0);
    svm.set_account(mock_mint.pubkey(), mint_account).unwrap();

    let mark_ix = anchor_instruction(
        program_id,
        "mark_as_minted",
        &[],
        vec![
            signer_meta(authority.pubkey()),   // authority
            writable_meta(mock_mint.pubkey()), // mint
            writable_meta(nft_config_pda),     // nft_config
        ],
    );
    execute_tx(&mut svm, mark_ix, &authority, &[&authority]).unwrap();

    // Try to update after minting
    svm.expire_blockhash();
    let mut update_ix_data = Vec::new();
    update_ix_data.extend_from_slice(&serialize_string("New Name"));
    update_ix_data.extend_from_slice(&serialize_string("NEW"));
    update_ix_data.extend_from_slice(&serialize_string("https://new.com"));

    let update_ix = anchor_instruction(
        program_id,
        "update_nft_config",
        &update_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(nft_config_pda),
        ],
    );

    let result = execute_tx(&mut svm, update_ix, &authority, &[&authority]);
    assert!(result.is_err(), "Update after minting should fail");
}

// =============================================================================
// MARK AS MINTED TESTS
// =============================================================================

#[test]
fn test_mark_as_minted() {
    let mut svm = load_nft_mint_program();
    let program_id = nft_mint_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let nft_id: u64 = 1;
    let (nft_config_pda, _) = derive_nft_config_pda(nft_id);

    // Create NFT config
    let mut create_ix_data = Vec::new();
    create_ix_data.extend_from_slice(&nft_id.to_le_bytes());
    create_ix_data.extend_from_slice(&serialize_string("Test NFT"));
    create_ix_data.extend_from_slice(&serialize_string("TNFT"));
    create_ix_data.extend_from_slice(&serialize_string("https://example.com"));

    let create_ix = anchor_instruction(
        program_id,
        "create_nft_config",
        &create_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(nft_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Verify not minted
    let nft_config_account = svm.get_account(&nft_config_pda).unwrap();
    let (_, _, _, _, _, _, is_minted, _) = read_nft_config(&nft_config_account.data);
    assert!(!is_minted, "Should not be minted initially");

    // Create mock mint and mark as minted
    svm.expire_blockhash();
    let mock_mint = Keypair::new();
    let mint_account = create_mint_account(&authority.pubkey(), 0);
    svm.set_account(mock_mint.pubkey(), mint_account).unwrap();

    let mark_ix = anchor_instruction(
        program_id,
        "mark_as_minted",
        &[],
        vec![
            signer_meta(authority.pubkey()),   // authority
            writable_meta(mock_mint.pubkey()), // mint
            writable_meta(nft_config_pda),     // nft_config
        ],
    );

    let result = execute_tx(&mut svm, mark_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "Mark as minted should succeed: {:?}", result);

    // Verify minted state
    let nft_config_account = svm.get_account(&nft_config_pda).unwrap();
    let (_, _, stored_mint, _, _, _, is_minted, _) = read_nft_config(&nft_config_account.data);

    assert!(is_minted, "Should be minted now");
    assert_eq!(stored_mint, mock_mint.pubkey(), "Mint address should be stored");
}

#[test]
fn test_mark_as_minted_unauthorized_fails() {
    let mut svm = load_nft_mint_program();
    let program_id = nft_mint_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let attacker = funded_keypair_10_sol(&mut svm);
    let nft_id: u64 = 1;
    let (nft_config_pda, _) = derive_nft_config_pda(nft_id);

    // Create NFT config by authority
    let mut create_ix_data = Vec::new();
    create_ix_data.extend_from_slice(&nft_id.to_le_bytes());
    create_ix_data.extend_from_slice(&serialize_string("Test NFT"));
    create_ix_data.extend_from_slice(&serialize_string("TNFT"));
    create_ix_data.extend_from_slice(&serialize_string("https://example.com"));

    let create_ix = anchor_instruction(
        program_id,
        "create_nft_config",
        &create_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(nft_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Attacker tries to mark as minted
    svm.expire_blockhash();
    let fake_mint = Keypair::new();
    let mint_account = create_mint_account(&attacker.pubkey(), 0);
    svm.set_account(fake_mint.pubkey(), mint_account).unwrap();

    let mark_ix = anchor_instruction(
        program_id,
        "mark_as_minted",
        &[],
        vec![
            signer_meta(attacker.pubkey()),   // attacker signs
            writable_meta(fake_mint.pubkey()), // fake mint
            writable_meta(nft_config_pda),     // but nft_config.authority != attacker
        ],
    );

    let result = execute_tx(&mut svm, mark_ix, &attacker, &[&attacker]);
    assert!(result.is_err(), "Unauthorized mark should fail");
}

#[test]
fn test_mark_as_minted_twice_fails() {
    let mut svm = load_nft_mint_program();
    let program_id = nft_mint_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let nft_id: u64 = 1;
    let (nft_config_pda, _) = derive_nft_config_pda(nft_id);

    // Create NFT config
    let mut create_ix_data = Vec::new();
    create_ix_data.extend_from_slice(&nft_id.to_le_bytes());
    create_ix_data.extend_from_slice(&serialize_string("Test NFT"));
    create_ix_data.extend_from_slice(&serialize_string("TNFT"));
    create_ix_data.extend_from_slice(&serialize_string("https://example.com"));

    let create_ix = anchor_instruction(
        program_id,
        "create_nft_config",
        &create_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(nft_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Mark as minted first time
    svm.expire_blockhash();
    let mock_mint = Keypair::new();
    let mint_account = create_mint_account(&authority.pubkey(), 0);
    svm.set_account(mock_mint.pubkey(), mint_account).unwrap();

    let mark_ix_1 = anchor_instruction(
        program_id,
        "mark_as_minted",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mock_mint.pubkey()),
            writable_meta(nft_config_pda),
        ],
    );
    execute_tx(&mut svm, mark_ix_1, &authority, &[&authority]).unwrap();

    // Try to mark as minted again
    svm.expire_blockhash();
    let another_mint = Keypair::new();
    let mint_account = create_mint_account(&authority.pubkey(), 0);
    svm.set_account(another_mint.pubkey(), mint_account).unwrap();

    let mark_ix_2 = anchor_instruction(
        program_id,
        "mark_as_minted",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(another_mint.pubkey()),
            writable_meta(nft_config_pda),
        ],
    );

    let result = execute_tx(&mut svm, mark_ix_2, &authority, &[&authority]);
    assert!(result.is_err(), "Mark as minted twice should fail");
}

// =============================================================================
// GET NFT INFO TESTS
// =============================================================================

#[test]
fn test_get_nft_info() {
    let mut svm = load_nft_mint_program();
    let program_id = nft_mint_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let nft_id: u64 = 42;
    let (nft_config_pda, _) = derive_nft_config_pda(nft_id);

    let name = "Special NFT";
    let symbol = "SNFT";
    let uri = "https://special.example.com/metadata.json";

    // Create NFT config
    let mut create_ix_data = Vec::new();
    create_ix_data.extend_from_slice(&nft_id.to_le_bytes());
    create_ix_data.extend_from_slice(&serialize_string(name));
    create_ix_data.extend_from_slice(&serialize_string(symbol));
    create_ix_data.extend_from_slice(&serialize_string(uri));

    let create_ix = anchor_instruction(
        program_id,
        "create_nft_config",
        &create_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(nft_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Get NFT info (instruction just logs data)
    svm.expire_blockhash();
    let info_ix = anchor_instruction(
        program_id,
        "get_nft_info",
        &[],
        vec![writable_meta(nft_config_pda)],
    );

    let result = execute_tx(&mut svm, info_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "Get NFT info should succeed: {:?}", result);
}

// =============================================================================
// FULL WORKFLOW TESTS
// =============================================================================

#[test]
fn test_full_nft_workflow() {
    let mut svm = load_nft_mint_program();
    let program_id = nft_mint_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let nft_id: u64 = 1;
    let (nft_config_pda, _) = derive_nft_config_pda(nft_id);

    // 1. Create NFT config
    let mut create_ix_data = Vec::new();
    create_ix_data.extend_from_slice(&nft_id.to_le_bytes());
    create_ix_data.extend_from_slice(&serialize_string("Draft NFT"));
    create_ix_data.extend_from_slice(&serialize_string("DRFT"));
    create_ix_data.extend_from_slice(&serialize_string("https://draft.example.com"));

    let create_ix = anchor_instruction(
        program_id,
        "create_nft_config",
        &create_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(nft_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // 2. Update NFT config before minting
    svm.expire_blockhash();
    let mut update_ix_data = Vec::new();
    update_ix_data.extend_from_slice(&serialize_string("Final NFT"));
    update_ix_data.extend_from_slice(&serialize_string("FNFT"));
    update_ix_data.extend_from_slice(&serialize_string("https://final.example.com"));

    let update_ix = anchor_instruction(
        program_id,
        "update_nft_config",
        &update_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(nft_config_pda),
        ],
    );
    execute_tx(&mut svm, update_ix, &authority, &[&authority]).unwrap();

    // 3. Get info to verify update
    svm.expire_blockhash();
    let info_ix = anchor_instruction(
        program_id,
        "get_nft_info",
        &[],
        vec![writable_meta(nft_config_pda)],
    );
    execute_tx(&mut svm, info_ix, &authority, &[&authority]).unwrap();

    // 4. Mark as minted
    svm.expire_blockhash();
    let mock_mint = Keypair::new();
    let mint_account = create_mint_account(&authority.pubkey(), 0);
    svm.set_account(mock_mint.pubkey(), mint_account).unwrap();

    let mark_ix = anchor_instruction(
        program_id,
        "mark_as_minted",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mock_mint.pubkey()),
            writable_meta(nft_config_pda),
        ],
    );
    execute_tx(&mut svm, mark_ix, &authority, &[&authority]).unwrap();

    // Verify final state
    let nft_config_account = svm.get_account(&nft_config_pda).unwrap();
    let (
        stored_authority,
        stored_nft_id,
        stored_mint,
        stored_name,
        stored_symbol,
        stored_uri,
        is_minted,
        _bump,
    ) = read_nft_config(&nft_config_account.data);

    assert_eq!(stored_authority, authority.pubkey(), "Authority should match");
    assert_eq!(stored_nft_id, nft_id, "NFT ID should match");
    assert_eq!(stored_mint, mock_mint.pubkey(), "Mint should be recorded");
    assert_eq!(stored_name, "Final NFT", "Name should be final");
    assert_eq!(stored_symbol, "FNFT", "Symbol should be final");
    assert_eq!(stored_uri, "https://final.example.com", "URI should be final");
    assert!(is_minted, "Should be minted");
}

#[test]
fn test_pda_determinism() {
    // Verify PDA derivation is deterministic
    let nft_id: u64 = 12345;
    let (pda1, bump1) = derive_nft_config_pda(nft_id);
    let (pda2, bump2) = derive_nft_config_pda(nft_id);

    assert_eq!(pda1, pda2, "PDA should be deterministic");
    assert_eq!(bump1, bump2, "Bump should be deterministic");

    // Different IDs should produce different PDAs
    let (pda3, _) = derive_nft_config_pda(54321);
    assert_ne!(pda1, pda3, "Different IDs should produce different PDAs");
}
