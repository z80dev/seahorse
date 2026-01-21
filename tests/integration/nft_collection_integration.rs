//! LiteSVM integration tests for the Seahorse NFT collection program
//!
//! These tests verify the NFT collection management and verified membership functionality.
//! Note: Seahorse doesn't have native Metaplex Token Metadata support, so these tests
//! focus on the collection tracking functionality (similar to the Token-2022 pattern).
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile nft_collection.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::str::FromStr;

/// NFT collection program ID (from declare_id!)
fn nft_collection_program_id() -> Pubkey {
    Pubkey::from_str("NFTCoL1ect1on111111111111111111111111111111").unwrap()
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the nft_collection program into LiteSVM
fn load_nft_collection_program() -> litesvm::LiteSVM {
    let program_id = nft_collection_program_id();
    let program_bytes = std::fs::read("../../target/deploy/nft_collection.so")
        .expect("Failed to read nft_collection.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);
    svm
}

/// Derive collection PDA
fn derive_collection_pda(collection_id: u64) -> (Pubkey, u8) {
    find_pda(
        &[b"collection", &collection_id.to_le_bytes()],
        &nft_collection_program_id(),
    )
}

/// Derive member PDA
fn derive_member_pda(collection: &Pubkey, nft_mint: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"member", collection.as_ref(), nft_mint.as_ref()],
        &nft_collection_program_id(),
    )
}

/// Read Collection fields from account data
/// Layout: discriminator (8) + authority (32) + collection_id (8) + collection_mint (32) +
///         name (4+len) + symbol (4+len) + uri (4+len) + total_nfts (8) + verified_nfts (8) +
///         is_finalized (1) + bump (1)
fn read_collection(data: &[u8]) -> (Pubkey, u64, Pubkey, String, String, String, u64, u64, bool, u8) {
    let authority = Pubkey::new_from_array(data[8..40].try_into().unwrap());
    let collection_id = u64::from_le_bytes(data[40..48].try_into().unwrap());
    let collection_mint = Pubkey::new_from_array(data[48..80].try_into().unwrap());

    // Borsh strings: 4-byte length prefix (u32 LE) + bytes
    let name_len = u32::from_le_bytes(data[80..84].try_into().unwrap()) as usize;
    let name = String::from_utf8(data[84..84 + name_len].to_vec()).unwrap_or_default();

    let symbol_offset = 84 + name_len;
    let symbol_len = u32::from_le_bytes(data[symbol_offset..symbol_offset + 4].try_into().unwrap()) as usize;
    let symbol = String::from_utf8(data[symbol_offset + 4..symbol_offset + 4 + symbol_len].to_vec()).unwrap_or_default();

    let uri_offset = symbol_offset + 4 + symbol_len;
    let uri_len = u32::from_le_bytes(data[uri_offset..uri_offset + 4].try_into().unwrap()) as usize;
    let uri = String::from_utf8(data[uri_offset + 4..uri_offset + 4 + uri_len].to_vec()).unwrap_or_default();

    let nums_offset = uri_offset + 4 + uri_len;
    let total_nfts = u64::from_le_bytes(data[nums_offset..nums_offset + 8].try_into().unwrap());
    let verified_nfts = u64::from_le_bytes(data[nums_offset + 8..nums_offset + 16].try_into().unwrap());

    let is_finalized_offset = nums_offset + 16;
    let is_finalized = data[is_finalized_offset] != 0;

    let bump_offset = is_finalized_offset + 1;
    let bump = data[bump_offset];

    (authority, collection_id, collection_mint, name, symbol, uri, total_nfts, verified_nfts, is_finalized, bump)
}

/// Read CollectionMember fields from account data
/// Layout: discriminator (8) + collection (32) + nft_mint (32) + is_verified (1) + bump (1)
fn read_member(data: &[u8]) -> (Pubkey, Pubkey, bool, u8) {
    let collection = Pubkey::new_from_array(data[8..40].try_into().unwrap());
    let nft_mint = Pubkey::new_from_array(data[40..72].try_into().unwrap());
    let is_verified = data[72] != 0;
    let bump = data[73];

    (collection, nft_mint, is_verified, bump)
}

/// Serialize a string as Borsh (4-byte length prefix + bytes)
fn serialize_string(s: &str) -> Vec<u8> {
    let mut result = Vec::new();
    result.extend_from_slice(&(s.len() as u32).to_le_bytes());
    result.extend_from_slice(s.as_bytes());
    result
}

// =============================================================================
// CREATE COLLECTION TESTS
// =============================================================================

#[test]
fn test_create_collection() {
    let mut svm = load_nft_collection_program();
    let program_id = nft_collection_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let collection_id: u64 = 1;
    let (collection_pda, _) = derive_collection_pda(collection_id);

    let name = "Test Collection";
    let symbol = "TCOL";
    let uri = "https://example.com/collection.json";

    // Build instruction data: collection_id (u64) + name (string) + symbol (string) + uri (string)
    let mut ix_data = Vec::new();
    ix_data.extend_from_slice(&collection_id.to_le_bytes());
    ix_data.extend_from_slice(&serialize_string(name));
    ix_data.extend_from_slice(&serialize_string(symbol));
    ix_data.extend_from_slice(&serialize_string(uri));

    let ix = anchor_instruction(
        program_id,
        "create_collection",
        &ix_data,
        vec![
            signer_meta(authority.pubkey()),           // authority
            writable_meta(collection_pda),             // collection
            readonly_meta(solana_sdk_ids::sysvar::rent::id()), // rent
            readonly_meta(system_program::id()),       // system_program
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(
        result.is_ok(),
        "Create collection should succeed: {:?}",
        result
    );

    // Verify collection was created
    let collection_account = svm
        .get_account(&collection_pda)
        .expect("Collection should exist");
    assert_eq!(
        collection_account.owner, program_id,
        "Collection should be owned by program"
    );

    let (
        stored_authority,
        stored_collection_id,
        stored_collection_mint,
        stored_name,
        stored_symbol,
        stored_uri,
        total_nfts,
        verified_nfts,
        is_finalized,
        _bump,
    ) = read_collection(&collection_account.data);

    assert_eq!(stored_authority, authority.pubkey(), "Authority should match");
    assert_eq!(stored_collection_id, collection_id, "Collection ID should match");
    // Collection mint is placeholder (set to authority until set_collection_mint)
    assert_eq!(stored_collection_mint, authority.pubkey(), "Collection mint should be placeholder");
    assert_eq!(stored_name, name, "Name should match");
    assert_eq!(stored_symbol, symbol, "Symbol should match");
    assert_eq!(stored_uri, uri, "URI should match");
    assert_eq!(total_nfts, 0, "Total NFTs should be 0");
    assert_eq!(verified_nfts, 0, "Verified NFTs should be 0");
    assert!(!is_finalized, "Should not be finalized initially");
}

#[test]
fn test_create_collection_different_ids() {
    let mut svm = load_nft_collection_program();
    let program_id = nft_collection_program_id();

    let authority = funded_keypair_10_sol(&mut svm);

    // Create first collection
    let collection_id_1: u64 = 100;
    let (collection_pda_1, _) = derive_collection_pda(collection_id_1);

    let mut ix_data_1 = Vec::new();
    ix_data_1.extend_from_slice(&collection_id_1.to_le_bytes());
    ix_data_1.extend_from_slice(&serialize_string("First Collection"));
    ix_data_1.extend_from_slice(&serialize_string("COL1"));
    ix_data_1.extend_from_slice(&serialize_string("https://example.com/1.json"));

    let ix_1 = anchor_instruction(
        program_id,
        "create_collection",
        &ix_data_1,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda_1),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, ix_1, &authority, &[&authority]).unwrap();

    // Create second collection with different ID
    svm.expire_blockhash();
    let collection_id_2: u64 = 200;
    let (collection_pda_2, _) = derive_collection_pda(collection_id_2);

    let mut ix_data_2 = Vec::new();
    ix_data_2.extend_from_slice(&collection_id_2.to_le_bytes());
    ix_data_2.extend_from_slice(&serialize_string("Second Collection"));
    ix_data_2.extend_from_slice(&serialize_string("COL2"));
    ix_data_2.extend_from_slice(&serialize_string("https://example.com/2.json"));

    let ix_2 = anchor_instruction(
        program_id,
        "create_collection",
        &ix_data_2,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda_2),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix_2, &authority, &[&authority]);
    assert!(result.is_ok(), "Should create second collection");

    // Verify both exist
    assert!(svm.get_account(&collection_pda_1).is_some(), "First collection should exist");
    assert!(svm.get_account(&collection_pda_2).is_some(), "Second collection should exist");
    assert_ne!(collection_pda_1, collection_pda_2, "PDAs should be different");
}

#[test]
fn test_create_collection_name_too_long_fails() {
    let mut svm = load_nft_collection_program();
    let program_id = nft_collection_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let collection_id: u64 = 1;
    let (collection_pda, _) = derive_collection_pda(collection_id);

    // Name exceeds 32 characters
    let long_name = "This name is way too long for a collection name!";
    assert!(long_name.len() > 32);

    let mut ix_data = Vec::new();
    ix_data.extend_from_slice(&collection_id.to_le_bytes());
    ix_data.extend_from_slice(&serialize_string(long_name));
    ix_data.extend_from_slice(&serialize_string("SYM"));
    ix_data.extend_from_slice(&serialize_string("https://example.com"));

    let ix = anchor_instruction(
        program_id,
        "create_collection",
        &ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_err(), "Name too long should fail");
}

#[test]
fn test_create_collection_symbol_too_long_fails() {
    let mut svm = load_nft_collection_program();
    let program_id = nft_collection_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let collection_id: u64 = 1;
    let (collection_pda, _) = derive_collection_pda(collection_id);

    // Symbol exceeds 10 characters
    let long_symbol = "VERYLONGSYM";
    assert!(long_symbol.len() > 10);

    let mut ix_data = Vec::new();
    ix_data.extend_from_slice(&collection_id.to_le_bytes());
    ix_data.extend_from_slice(&serialize_string("Test Collection"));
    ix_data.extend_from_slice(&serialize_string(long_symbol));
    ix_data.extend_from_slice(&serialize_string("https://example.com"));

    let ix = anchor_instruction(
        program_id,
        "create_collection",
        &ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_err(), "Symbol too long should fail");
}

// =============================================================================
// UPDATE COLLECTION TESTS
// =============================================================================

#[test]
fn test_update_collection() {
    let mut svm = load_nft_collection_program();
    let program_id = nft_collection_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let collection_id: u64 = 1;
    let (collection_pda, _) = derive_collection_pda(collection_id);

    // Create collection
    let mut create_ix_data = Vec::new();
    create_ix_data.extend_from_slice(&collection_id.to_le_bytes());
    create_ix_data.extend_from_slice(&serialize_string("Original Name"));
    create_ix_data.extend_from_slice(&serialize_string("ORIG"));
    create_ix_data.extend_from_slice(&serialize_string("https://old.example.com"));

    let create_ix = anchor_instruction(
        program_id,
        "create_collection",
        &create_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Update collection
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
        "update_collection",
        &update_ix_data,
        vec![
            signer_meta(authority.pubkey()), // authority
            writable_meta(collection_pda),   // collection
        ],
    );

    let result = execute_tx(&mut svm, update_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "Update should succeed: {:?}", result);

    // Verify updated values
    let collection_account = svm.get_account(&collection_pda).unwrap();
    let (_, _, _, stored_name, stored_symbol, stored_uri, _, _, _, _) = read_collection(&collection_account.data);

    assert_eq!(stored_name, new_name, "Name should be updated");
    assert_eq!(stored_symbol, new_symbol, "Symbol should be updated");
    assert_eq!(stored_uri, new_uri, "URI should be updated");
}

#[test]
fn test_update_collection_unauthorized_fails() {
    let mut svm = load_nft_collection_program();
    let program_id = nft_collection_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let attacker = funded_keypair_10_sol(&mut svm);
    let collection_id: u64 = 1;
    let (collection_pda, _) = derive_collection_pda(collection_id);

    // Create collection by authority
    let mut create_ix_data = Vec::new();
    create_ix_data.extend_from_slice(&collection_id.to_le_bytes());
    create_ix_data.extend_from_slice(&serialize_string("My Collection"));
    create_ix_data.extend_from_slice(&serialize_string("MCOL"));
    create_ix_data.extend_from_slice(&serialize_string("https://example.com"));

    let create_ix = anchor_instruction(
        program_id,
        "create_collection",
        &create_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
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
        "update_collection",
        &update_ix_data,
        vec![
            signer_meta(attacker.pubkey()), // attacker signs
            writable_meta(collection_pda),  // but collection.authority != attacker
        ],
    );

    let result = execute_tx(&mut svm, update_ix, &attacker, &[&attacker]);
    assert!(result.is_err(), "Unauthorized update should fail");
}

// =============================================================================
// FINALIZE COLLECTION TESTS
// =============================================================================

#[test]
fn test_finalize_collection() {
    let mut svm = load_nft_collection_program();
    let program_id = nft_collection_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let collection_id: u64 = 1;
    let (collection_pda, _) = derive_collection_pda(collection_id);

    // Create collection
    let mut create_ix_data = Vec::new();
    create_ix_data.extend_from_slice(&collection_id.to_le_bytes());
    create_ix_data.extend_from_slice(&serialize_string("Test Collection"));
    create_ix_data.extend_from_slice(&serialize_string("TCOL"));
    create_ix_data.extend_from_slice(&serialize_string("https://example.com"));

    let create_ix = anchor_instruction(
        program_id,
        "create_collection",
        &create_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Finalize collection
    svm.expire_blockhash();
    let finalize_ix = anchor_instruction(
        program_id,
        "finalize_collection",
        &[],
        vec![
            signer_meta(authority.pubkey()), // authority
            writable_meta(collection_pda),   // collection
        ],
    );

    let result = execute_tx(&mut svm, finalize_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "Finalize should succeed: {:?}", result);

    // Verify finalized
    let collection_account = svm.get_account(&collection_pda).unwrap();
    let (_, _, _, _, _, _, _, _, is_finalized, _) = read_collection(&collection_account.data);
    assert!(is_finalized, "Should be finalized");
}

#[test]
fn test_update_collection_after_finalization_fails() {
    let mut svm = load_nft_collection_program();
    let program_id = nft_collection_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let collection_id: u64 = 1;
    let (collection_pda, _) = derive_collection_pda(collection_id);

    // Create and finalize collection
    let mut create_ix_data = Vec::new();
    create_ix_data.extend_from_slice(&collection_id.to_le_bytes());
    create_ix_data.extend_from_slice(&serialize_string("Test Collection"));
    create_ix_data.extend_from_slice(&serialize_string("TCOL"));
    create_ix_data.extend_from_slice(&serialize_string("https://example.com"));

    let create_ix = anchor_instruction(
        program_id,
        "create_collection",
        &create_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    svm.expire_blockhash();
    let finalize_ix = anchor_instruction(
        program_id,
        "finalize_collection",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
        ],
    );
    execute_tx(&mut svm, finalize_ix, &authority, &[&authority]).unwrap();

    // Try to update after finalization
    svm.expire_blockhash();
    let mut update_ix_data = Vec::new();
    update_ix_data.extend_from_slice(&serialize_string("New Name"));
    update_ix_data.extend_from_slice(&serialize_string("NEW"));
    update_ix_data.extend_from_slice(&serialize_string("https://new.com"));

    let update_ix = anchor_instruction(
        program_id,
        "update_collection",
        &update_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
        ],
    );

    let result = execute_tx(&mut svm, update_ix, &authority, &[&authority]);
    assert!(result.is_err(), "Update after finalization should fail");
}

// =============================================================================
// ADD NFT TO COLLECTION TESTS
// =============================================================================

#[test]
fn test_add_nft_to_collection() {
    let mut svm = load_nft_collection_program();
    let program_id = nft_collection_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let collection_id: u64 = 1;
    let (collection_pda, _) = derive_collection_pda(collection_id);

    // Create collection
    let mut create_ix_data = Vec::new();
    create_ix_data.extend_from_slice(&collection_id.to_le_bytes());
    create_ix_data.extend_from_slice(&serialize_string("Test Collection"));
    create_ix_data.extend_from_slice(&serialize_string("TCOL"));
    create_ix_data.extend_from_slice(&serialize_string("https://example.com"));

    let create_ix = anchor_instruction(
        program_id,
        "create_collection",
        &create_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Create mock NFT mint and add to collection
    svm.expire_blockhash();
    let nft_mint = Keypair::new();
    let mint_account = create_mint_account(&authority.pubkey(), 0);
    svm.set_account(nft_mint.pubkey(), mint_account).unwrap();

    let (member_pda, _) = derive_member_pda(&collection_pda, &nft_mint.pubkey());

    let add_ix = anchor_instruction(
        program_id,
        "add_nft_to_collection",
        &[],
        vec![
            signer_meta(authority.pubkey()),   // authority
            writable_meta(collection_pda),     // collection
            writable_meta(nft_mint.pubkey()),  // nft_mint
            writable_meta(member_pda),         // member
            readonly_meta(solana_sdk_ids::sysvar::rent::id()), // rent
            readonly_meta(system_program::id()), // system_program
        ],
    );

    let result = execute_tx(&mut svm, add_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "Add NFT to collection should succeed: {:?}", result);

    // Verify member account
    let member_account = svm.get_account(&member_pda).expect("Member should exist");
    let (stored_collection, stored_nft_mint, is_verified, _bump) = read_member(&member_account.data);

    assert_eq!(stored_collection, collection_pda, "Collection should match");
    assert_eq!(stored_nft_mint, nft_mint.pubkey(), "NFT mint should match");
    assert!(!is_verified, "Should not be verified initially");

    // Verify collection total_nfts incremented
    let collection_account = svm.get_account(&collection_pda).unwrap();
    let (_, _, _, _, _, _, total_nfts, _, _, _) = read_collection(&collection_account.data);
    assert_eq!(total_nfts, 1, "Total NFTs should be 1");
}

#[test]
fn test_add_multiple_nfts_to_collection() {
    let mut svm = load_nft_collection_program();
    let program_id = nft_collection_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let collection_id: u64 = 1;
    let (collection_pda, _) = derive_collection_pda(collection_id);

    // Create collection
    let mut create_ix_data = Vec::new();
    create_ix_data.extend_from_slice(&collection_id.to_le_bytes());
    create_ix_data.extend_from_slice(&serialize_string("Test Collection"));
    create_ix_data.extend_from_slice(&serialize_string("TCOL"));
    create_ix_data.extend_from_slice(&serialize_string("https://example.com"));

    let create_ix = anchor_instruction(
        program_id,
        "create_collection",
        &create_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Add 3 NFTs
    for i in 0..3 {
        svm.expire_blockhash();
        let nft_mint = Keypair::new();
        let mint_account = create_mint_account(&authority.pubkey(), 0);
        svm.set_account(nft_mint.pubkey(), mint_account).unwrap();

        let (member_pda, _) = derive_member_pda(&collection_pda, &nft_mint.pubkey());

        let add_ix = anchor_instruction(
            program_id,
            "add_nft_to_collection",
            &[],
            vec![
                signer_meta(authority.pubkey()),
                writable_meta(collection_pda),
                writable_meta(nft_mint.pubkey()),
                writable_meta(member_pda),
                readonly_meta(solana_sdk_ids::sysvar::rent::id()),
                readonly_meta(system_program::id()),
            ],
        );

        let result = execute_tx(&mut svm, add_ix, &authority, &[&authority]);
        assert!(result.is_ok(), "Add NFT {} should succeed", i);
    }

    // Verify collection total_nfts
    let collection_account = svm.get_account(&collection_pda).unwrap();
    let (_, _, _, _, _, _, total_nfts, _, _, _) = read_collection(&collection_account.data);
    assert_eq!(total_nfts, 3, "Total NFTs should be 3");
}

// =============================================================================
// VERIFY NFT TESTS
// =============================================================================

#[test]
fn test_verify_nft() {
    let mut svm = load_nft_collection_program();
    let program_id = nft_collection_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let collection_id: u64 = 1;
    let (collection_pda, _) = derive_collection_pda(collection_id);

    // Create collection
    let mut create_ix_data = Vec::new();
    create_ix_data.extend_from_slice(&collection_id.to_le_bytes());
    create_ix_data.extend_from_slice(&serialize_string("Test Collection"));
    create_ix_data.extend_from_slice(&serialize_string("TCOL"));
    create_ix_data.extend_from_slice(&serialize_string("https://example.com"));

    let create_ix = anchor_instruction(
        program_id,
        "create_collection",
        &create_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Add NFT
    svm.expire_blockhash();
    let nft_mint = Keypair::new();
    let mint_account = create_mint_account(&authority.pubkey(), 0);
    svm.set_account(nft_mint.pubkey(), mint_account).unwrap();

    let (member_pda, _) = derive_member_pda(&collection_pda, &nft_mint.pubkey());

    let add_ix = anchor_instruction(
        program_id,
        "add_nft_to_collection",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
            writable_meta(nft_mint.pubkey()),
            writable_meta(member_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, add_ix, &authority, &[&authority]).unwrap();

    // Verify NFT
    svm.expire_blockhash();
    let verify_ix = anchor_instruction(
        program_id,
        "verify_nft",
        &[],
        vec![
            signer_meta(authority.pubkey()), // authority
            writable_meta(collection_pda),   // collection
            writable_meta(member_pda),       // member
        ],
    );

    let result = execute_tx(&mut svm, verify_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "Verify NFT should succeed: {:?}", result);

    // Verify member is_verified
    let member_account = svm.get_account(&member_pda).unwrap();
    let (_, _, is_verified, _) = read_member(&member_account.data);
    assert!(is_verified, "NFT should be verified");

    // Verify collection verified_nfts incremented
    let collection_account = svm.get_account(&collection_pda).unwrap();
    let (_, _, _, _, _, _, _, verified_nfts, _, _) = read_collection(&collection_account.data);
    assert_eq!(verified_nfts, 1, "Verified NFTs should be 1");
}

#[test]
fn test_verify_nft_unauthorized_fails() {
    let mut svm = load_nft_collection_program();
    let program_id = nft_collection_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let attacker = funded_keypair_10_sol(&mut svm);
    let collection_id: u64 = 1;
    let (collection_pda, _) = derive_collection_pda(collection_id);

    // Create collection
    let mut create_ix_data = Vec::new();
    create_ix_data.extend_from_slice(&collection_id.to_le_bytes());
    create_ix_data.extend_from_slice(&serialize_string("Test Collection"));
    create_ix_data.extend_from_slice(&serialize_string("TCOL"));
    create_ix_data.extend_from_slice(&serialize_string("https://example.com"));

    let create_ix = anchor_instruction(
        program_id,
        "create_collection",
        &create_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Add NFT
    svm.expire_blockhash();
    let nft_mint = Keypair::new();
    let mint_account = create_mint_account(&authority.pubkey(), 0);
    svm.set_account(nft_mint.pubkey(), mint_account).unwrap();

    let (member_pda, _) = derive_member_pda(&collection_pda, &nft_mint.pubkey());

    let add_ix = anchor_instruction(
        program_id,
        "add_nft_to_collection",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
            writable_meta(nft_mint.pubkey()),
            writable_meta(member_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, add_ix, &authority, &[&authority]).unwrap();

    // Attacker tries to verify
    svm.expire_blockhash();
    let verify_ix = anchor_instruction(
        program_id,
        "verify_nft",
        &[],
        vec![
            signer_meta(attacker.pubkey()), // attacker signs
            writable_meta(collection_pda),  // but collection.authority != attacker
            writable_meta(member_pda),
        ],
    );

    let result = execute_tx(&mut svm, verify_ix, &attacker, &[&attacker]);
    assert!(result.is_err(), "Unauthorized verify should fail");
}

#[test]
fn test_verify_nft_twice_fails() {
    let mut svm = load_nft_collection_program();
    let program_id = nft_collection_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let collection_id: u64 = 1;
    let (collection_pda, _) = derive_collection_pda(collection_id);

    // Create collection
    let mut create_ix_data = Vec::new();
    create_ix_data.extend_from_slice(&collection_id.to_le_bytes());
    create_ix_data.extend_from_slice(&serialize_string("Test Collection"));
    create_ix_data.extend_from_slice(&serialize_string("TCOL"));
    create_ix_data.extend_from_slice(&serialize_string("https://example.com"));

    let create_ix = anchor_instruction(
        program_id,
        "create_collection",
        &create_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Add NFT
    svm.expire_blockhash();
    let nft_mint = Keypair::new();
    let mint_account = create_mint_account(&authority.pubkey(), 0);
    svm.set_account(nft_mint.pubkey(), mint_account).unwrap();

    let (member_pda, _) = derive_member_pda(&collection_pda, &nft_mint.pubkey());

    let add_ix = anchor_instruction(
        program_id,
        "add_nft_to_collection",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
            writable_meta(nft_mint.pubkey()),
            writable_meta(member_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, add_ix, &authority, &[&authority]).unwrap();

    // Verify NFT first time
    svm.expire_blockhash();
    let verify_ix_1 = anchor_instruction(
        program_id,
        "verify_nft",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
            writable_meta(member_pda),
        ],
    );
    execute_tx(&mut svm, verify_ix_1, &authority, &[&authority]).unwrap();

    // Try to verify again
    svm.expire_blockhash();
    let verify_ix_2 = anchor_instruction(
        program_id,
        "verify_nft",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
            writable_meta(member_pda),
        ],
    );

    let result = execute_tx(&mut svm, verify_ix_2, &authority, &[&authority]);
    assert!(result.is_err(), "Verify twice should fail");
}

// =============================================================================
// UNVERIFY NFT TESTS
// =============================================================================

#[test]
fn test_unverify_nft() {
    let mut svm = load_nft_collection_program();
    let program_id = nft_collection_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let collection_id: u64 = 1;
    let (collection_pda, _) = derive_collection_pda(collection_id);

    // Create collection
    let mut create_ix_data = Vec::new();
    create_ix_data.extend_from_slice(&collection_id.to_le_bytes());
    create_ix_data.extend_from_slice(&serialize_string("Test Collection"));
    create_ix_data.extend_from_slice(&serialize_string("TCOL"));
    create_ix_data.extend_from_slice(&serialize_string("https://example.com"));

    let create_ix = anchor_instruction(
        program_id,
        "create_collection",
        &create_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Add and verify NFT
    svm.expire_blockhash();
    let nft_mint = Keypair::new();
    let mint_account = create_mint_account(&authority.pubkey(), 0);
    svm.set_account(nft_mint.pubkey(), mint_account).unwrap();

    let (member_pda, _) = derive_member_pda(&collection_pda, &nft_mint.pubkey());

    let add_ix = anchor_instruction(
        program_id,
        "add_nft_to_collection",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
            writable_meta(nft_mint.pubkey()),
            writable_meta(member_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, add_ix, &authority, &[&authority]).unwrap();

    svm.expire_blockhash();
    let verify_ix = anchor_instruction(
        program_id,
        "verify_nft",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
            writable_meta(member_pda),
        ],
    );
    execute_tx(&mut svm, verify_ix, &authority, &[&authority]).unwrap();

    // Verify NFT is verified
    let member_account = svm.get_account(&member_pda).unwrap();
    let (_, _, is_verified, _) = read_member(&member_account.data);
    assert!(is_verified, "NFT should be verified");

    // Unverify NFT
    svm.expire_blockhash();
    let unverify_ix = anchor_instruction(
        program_id,
        "unverify_nft",
        &[],
        vec![
            signer_meta(authority.pubkey()), // authority
            writable_meta(collection_pda),   // collection
            writable_meta(member_pda),       // member
        ],
    );

    let result = execute_tx(&mut svm, unverify_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "Unverify NFT should succeed: {:?}", result);

    // Verify member is_verified is false
    let member_account = svm.get_account(&member_pda).unwrap();
    let (_, _, is_verified, _) = read_member(&member_account.data);
    assert!(!is_verified, "NFT should be unverified");

    // Verify collection verified_nfts decremented
    let collection_account = svm.get_account(&collection_pda).unwrap();
    let (_, _, _, _, _, _, _, verified_nfts, _, _) = read_collection(&collection_account.data);
    assert_eq!(verified_nfts, 0, "Verified NFTs should be 0");
}

#[test]
fn test_unverify_nft_not_verified_fails() {
    let mut svm = load_nft_collection_program();
    let program_id = nft_collection_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let collection_id: u64 = 1;
    let (collection_pda, _) = derive_collection_pda(collection_id);

    // Create collection
    let mut create_ix_data = Vec::new();
    create_ix_data.extend_from_slice(&collection_id.to_le_bytes());
    create_ix_data.extend_from_slice(&serialize_string("Test Collection"));
    create_ix_data.extend_from_slice(&serialize_string("TCOL"));
    create_ix_data.extend_from_slice(&serialize_string("https://example.com"));

    let create_ix = anchor_instruction(
        program_id,
        "create_collection",
        &create_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Add NFT (not verified)
    svm.expire_blockhash();
    let nft_mint = Keypair::new();
    let mint_account = create_mint_account(&authority.pubkey(), 0);
    svm.set_account(nft_mint.pubkey(), mint_account).unwrap();

    let (member_pda, _) = derive_member_pda(&collection_pda, &nft_mint.pubkey());

    let add_ix = anchor_instruction(
        program_id,
        "add_nft_to_collection",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
            writable_meta(nft_mint.pubkey()),
            writable_meta(member_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, add_ix, &authority, &[&authority]).unwrap();

    // Try to unverify without being verified first
    svm.expire_blockhash();
    let unverify_ix = anchor_instruction(
        program_id,
        "unverify_nft",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
            writable_meta(member_pda),
        ],
    );

    let result = execute_tx(&mut svm, unverify_ix, &authority, &[&authority]);
    assert!(result.is_err(), "Unverify unverified NFT should fail");
}

// =============================================================================
// FULL WORKFLOW TESTS
// =============================================================================

#[test]
fn test_full_collection_workflow() {
    let mut svm = load_nft_collection_program();
    let program_id = nft_collection_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let collection_id: u64 = 1;
    let (collection_pda, _) = derive_collection_pda(collection_id);

    // 1. Create collection
    let mut create_ix_data = Vec::new();
    create_ix_data.extend_from_slice(&collection_id.to_le_bytes());
    create_ix_data.extend_from_slice(&serialize_string("Cool NFT Collection"));
    create_ix_data.extend_from_slice(&serialize_string("COOL"));
    create_ix_data.extend_from_slice(&serialize_string("https://example.com/collection.json"));

    let create_ix = anchor_instruction(
        program_id,
        "create_collection",
        &create_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // 2. Update collection metadata
    svm.expire_blockhash();
    let mut update_ix_data = Vec::new();
    update_ix_data.extend_from_slice(&serialize_string("Amazing NFT Collection"));
    update_ix_data.extend_from_slice(&serialize_string("AMZN"));
    update_ix_data.extend_from_slice(&serialize_string("https://example.com/collection-v2.json"));

    let update_ix = anchor_instruction(
        program_id,
        "update_collection",
        &update_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
        ],
    );
    execute_tx(&mut svm, update_ix, &authority, &[&authority]).unwrap();

    // 3. Add 3 NFTs to collection
    let mut nft_mints = Vec::new();
    let mut member_pdas = Vec::new();

    for _ in 0..3 {
        svm.expire_blockhash();
        let nft_mint = Keypair::new();
        let mint_account = create_mint_account(&authority.pubkey(), 0);
        svm.set_account(nft_mint.pubkey(), mint_account).unwrap();

        let (member_pda, _) = derive_member_pda(&collection_pda, &nft_mint.pubkey());

        let add_ix = anchor_instruction(
            program_id,
            "add_nft_to_collection",
            &[],
            vec![
                signer_meta(authority.pubkey()),
                writable_meta(collection_pda),
                writable_meta(nft_mint.pubkey()),
                writable_meta(member_pda),
                readonly_meta(solana_sdk_ids::sysvar::rent::id()),
                readonly_meta(system_program::id()),
            ],
        );
        execute_tx(&mut svm, add_ix, &authority, &[&authority]).unwrap();

        nft_mints.push(nft_mint);
        member_pdas.push(member_pda);
    }

    // 4. Verify first 2 NFTs
    for member_pda in &member_pdas[..2] {
        svm.expire_blockhash();
        let verify_ix = anchor_instruction(
            program_id,
            "verify_nft",
            &[],
            vec![
                signer_meta(authority.pubkey()),
                writable_meta(collection_pda),
                writable_meta(*member_pda),
            ],
        );
        execute_tx(&mut svm, verify_ix, &authority, &[&authority]).unwrap();
    }

    // 5. Check state: 3 total, 2 verified
    let collection_account = svm.get_account(&collection_pda).unwrap();
    let (_, _, _, stored_name, stored_symbol, _, total_nfts, verified_nfts, is_finalized, _) = read_collection(&collection_account.data);

    assert_eq!(stored_name, "Amazing NFT Collection");
    assert_eq!(stored_symbol, "AMZN");
    assert_eq!(total_nfts, 3);
    assert_eq!(verified_nfts, 2);
    assert!(!is_finalized);

    // 6. Unverify first NFT
    svm.expire_blockhash();
    let unverify_ix = anchor_instruction(
        program_id,
        "unverify_nft",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
            writable_meta(member_pdas[0]),
        ],
    );
    execute_tx(&mut svm, unverify_ix, &authority, &[&authority]).unwrap();

    // 7. Finalize collection
    svm.expire_blockhash();
    let finalize_ix = anchor_instruction(
        program_id,
        "finalize_collection",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
        ],
    );
    execute_tx(&mut svm, finalize_ix, &authority, &[&authority]).unwrap();

    // 8. Verify final state: 3 total, 1 verified, finalized
    let collection_account = svm.get_account(&collection_pda).unwrap();
    let (_, _, _, _, _, _, total_nfts, verified_nfts, is_finalized, _) = read_collection(&collection_account.data);

    assert_eq!(total_nfts, 3, "Total NFTs should still be 3");
    assert_eq!(verified_nfts, 1, "Verified NFTs should be 1 (after unverify)");
    assert!(is_finalized, "Collection should be finalized");

    // 9. Verify can still add NFTs after finalization (only metadata locked)
    svm.expire_blockhash();
    let new_nft_mint = Keypair::new();
    let mint_account = create_mint_account(&authority.pubkey(), 0);
    svm.set_account(new_nft_mint.pubkey(), mint_account).unwrap();

    let (new_member_pda, _) = derive_member_pda(&collection_pda, &new_nft_mint.pubkey());

    let add_new_ix = anchor_instruction(
        program_id,
        "add_nft_to_collection",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collection_pda),
            writable_meta(new_nft_mint.pubkey()),
            writable_meta(new_member_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    let result = execute_tx(&mut svm, add_new_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "Can still add NFTs after finalization");

    // Verify total_nfts incremented
    let collection_account = svm.get_account(&collection_pda).unwrap();
    let (_, _, _, _, _, _, total_nfts, _, _, _) = read_collection(&collection_account.data);
    assert_eq!(total_nfts, 4, "Total NFTs should be 4");
}

#[test]
fn test_pda_determinism() {
    // Verify PDA derivation is deterministic
    let collection_id: u64 = 12345;
    let (pda1, bump1) = derive_collection_pda(collection_id);
    let (pda2, bump2) = derive_collection_pda(collection_id);

    assert_eq!(pda1, pda2, "Collection PDA should be deterministic");
    assert_eq!(bump1, bump2, "Collection bump should be deterministic");

    // Different IDs should produce different PDAs
    let (pda3, _) = derive_collection_pda(54321);
    assert_ne!(pda1, pda3, "Different IDs should produce different PDAs");

    // Verify member PDA determinism
    let collection = Pubkey::new_unique();
    let nft_mint = Pubkey::new_unique();
    let (member_pda1, member_bump1) = derive_member_pda(&collection, &nft_mint);
    let (member_pda2, member_bump2) = derive_member_pda(&collection, &nft_mint);

    assert_eq!(member_pda1, member_pda2, "Member PDA should be deterministic");
    assert_eq!(member_bump1, member_bump2, "Member bump should be deterministic");

    // Different nft_mint should produce different member PDAs
    let nft_mint_2 = Pubkey::new_unique();
    let (member_pda3, _) = derive_member_pda(&collection, &nft_mint_2);
    assert_ne!(member_pda1, member_pda3, "Different NFT mints should produce different member PDAs");
}
