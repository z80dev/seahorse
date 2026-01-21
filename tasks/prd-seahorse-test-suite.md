# PRD: Seahorse SDK 3.0 Upgrade & Comprehensive Test Suite

## Introduction

Upgrade Seahorse to Solana SDK 3.0, then extend the test infrastructure with modern Solana testing tools (LiteSVM + Mollusk) and an extensive library of 30+ example programs derived from existing idiomatic Anchor programs. The workflow: upgrade dependencies first, then find well-written Anchor programs, write Seahorse equivalents, and verify behavior parity (same inputs produce same state changes).

## Goals

- **Upgrade to Solana SDK 3.0** as the foundational prerequisite
- Update all dependencies (Anchor, mollusk-svm, litesvm) for SDK 3.0 compatibility
- Establish dual testing infrastructure: Mollusk for isolated unit tests, LiteSVM for integration tests
- Create 30+ Seahorse example programs covering all major Anchor use cases
- Verify behavior parity: same inputs produce same state changes as reference Anchor programs
- Support both Rust and TypeScript test harnesses
- Include comprehensive CPI testing with SPL Token, Associated Token, and Metaplex programs
- Provide a testing foundation that validates Seahorse compiler correctness

## User Stories

### Phase 0: Solana SDK 3.0 Upgrade (Prerequisites)

#### US-000: Upgrade solana-sdk to 3.0
**Description:** As a developer, I need to upgrade the core solana-sdk dependency to version 3.0 for compatibility with the latest Solana toolchain.

**Acceptance Criteria:**
- [ ] Update solana-sdk version to 3.0.x in Cargo.toml
- [ ] Update solana-program to compatible version
- [ ] Fix any breaking API changes in Seahorse compiler code
- [ ] `cargo build` succeeds with new SDK version
- [ ] `cargo test` passes
- [ ] Typecheck passes

#### US-001: Update Anchor dependencies for SDK 3.0
**Description:** As a developer, I need to update anchor-lang and anchor-spl to versions compatible with Solana SDK 3.0.

**Acceptance Criteria:**
- [ ] Update anchor-lang to latest version supporting SDK 3.0
- [ ] Update anchor-spl to matching version
- [ ] Update generated Cargo.toml template in `src/bin/cli/init.rs`
- [ ] Fix any breaking changes in generated code patterns
- [ ] Example programs compile with `anchor build`
- [ ] Typecheck passes

#### US-002: Update mollusk-svm for SDK 3.0
**Description:** As a developer, I need mollusk-svm compatible with Solana SDK 3.0 for unit testing.

**Acceptance Criteria:**
- [ ] Find mollusk-svm version compatible with solana-sdk 3.0
- [ ] Update mollusk-svm dependency in test workspace
- [ ] Verify mollusk-svm API compatibility (may have breaking changes)
- [ ] Update existing Mollusk test helpers if needed
- [ ] `cargo test` for Mollusk tests passes
- [ ] Typecheck passes

#### US-003: Update litesvm for SDK 3.0
**Description:** As a developer, I need litesvm compatible with Solana SDK 3.0 for integration testing.

**Acceptance Criteria:**
- [ ] Find litesvm version compatible with solana-sdk 3.0
- [ ] Update litesvm dependency in test workspace
- [ ] Update anchor-litesvm to matching version
- [ ] Verify litesvm API compatibility
- [ ] Update existing LiteSVM test helpers if needed
- [ ] `cargo test` for LiteSVM tests passes
- [ ] Typecheck passes

#### US-004: Verify example programs compile with SDK 3.0
**Description:** As a developer, I need to verify all existing Seahorse examples compile with the upgraded dependencies.

**Acceptance Criteria:**
- [ ] Compile `examples/calculator.py` through Seahorse
- [ ] Compile `examples/constants.py` through Seahorse
- [ ] Compile all other existing examples
- [ ] Generated Rust code builds with `anchor build`
- [ ] Fix any compilation errors from SDK 3.0 changes
- [ ] Typecheck passes

---

### Phase 1: Testing Infrastructure

#### US-005: Set up Mollusk test harness
**Description:** As a developer, I need a Mollusk-based test harness so I can run fast, isolated unit tests on individual instructions.

**Acceptance Criteria:**
- [ ] Add `mollusk-svm` dependency to test workspace
- [ ] Create `tests/unit/` directory structure
- [ ] Implement helper functions for common test setup (create accounts, load programs)
- [ ] Create example unit test demonstrating Mollusk usage
- [ ] Document Mollusk test patterns in tests/README.md
- [ ] `cargo test` runs Mollusk tests successfully

#### US-006: Set up LiteSVM Rust test harness
**Description:** As a developer, I need a LiteSVM-based Rust test harness for integration tests that simulate realistic transaction flows.

**Acceptance Criteria:**
- [ ] Add `litesvm` dependency to test workspace
- [ ] Create `tests/integration/` directory structure
- [ ] Implement test utilities for account creation, PDA derivation, transaction building
- [ ] Create example integration test demonstrating LiteSVM usage
- [ ] Tests can load compiled Seahorse programs (.so files)
- [ ] `cargo test` runs LiteSVM tests successfully

#### US-007: Set up TypeScript test harness
**Description:** As a developer, I need a TypeScript test harness using anchor-litesvm for tests that benefit from JS ergonomics.

**Acceptance Criteria:**
- [ ] Create `tests/ts/` directory with package.json
- [ ] Add `anchor-litesvm`, `@coral-xyz/anchor`, and test dependencies
- [ ] Implement TypeScript test utilities matching Rust helpers
- [ ] Create example TypeScript integration test
- [ ] `npm test` or `yarn test` runs TS tests successfully
- [ ] Document TS test setup in tests/ts/README.md

#### US-008: Create test program compilation pipeline
**Description:** As a developer, I need an automated pipeline that compiles Seahorse examples and reference Anchor programs for testing.

**Acceptance Criteria:**
- [ ] Script to compile all Seahorse examples to Rust
- [ ] Script to build compiled programs to .so files
- [ ] Script to build reference Anchor programs to .so files
- [ ] CI-friendly: can run in GitHub Actions
- [ ] Makefile or justfile with `make build-test-programs` target
- [ ] Built artifacts stored in predictable location (e.g., `target/deploy/`)

#### US-009: Implement behavior comparison framework
**Description:** As a developer, I need a framework to compare state changes between Seahorse and Anchor program executions.

**Acceptance Criteria:**
- [ ] Helper to execute same instruction on both Seahorse and Anchor versions
- [ ] Account state diffing: compare account data after execution
- [ ] Event emission comparison (if applicable)
- [ ] Clear error messages when behavior diverges
- [ ] Support for comparing multiple instructions in sequence
- [ ] Works with both Mollusk and LiteSVM harnesses

---

### Phase 2: Basic Program Examples (Foundational Patterns)

#### US-010: Counter program
**Description:** As a user, I want a simple counter example demonstrating basic account creation and mutation.

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/counter.py`
- [ ] Reference Anchor program: `tests/anchor-reference/counter/`
- [ ] Instructions: initialize, increment, decrement, set_value
- [ ] Unit tests verify each instruction in isolation (Mollusk)
- [ ] Integration test verifies full workflow (LiteSVM)
- [ ] Behavior parity verified against Anchor reference

#### US-011: Hello World with logging
**Description:** As a user, I want a minimal program demonstrating program logging and basic instruction handling.

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/hello_world.py`
- [ ] Reference Anchor program: `tests/anchor-reference/hello_world/`
- [ ] Instructions: say_hello (with msg! logging)
- [ ] Test verifies program executes without error
- [ ] Behavior parity verified

#### US-012: Calculator with enums
**Description:** As a user, I want a calculator example demonstrating enum usage and owner validation.

**Acceptance Criteria:**
- [ ] Enhance existing `examples/calculator.py` with tests
- [ ] Reference Anchor program: `tests/anchor-reference/calculator/`
- [ ] Test all operations: add, sub, mul, div
- [ ] Test owner validation (unauthorized access fails)
- [ ] Unit tests for each operation (Mollusk)
- [ ] Integration test for full calculator workflow (LiteSVM)
- [ ] Behavior parity verified

#### US-013: PDA derivation patterns
**Description:** As a user, I want examples demonstrating various PDA derivation patterns.

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/pda_patterns.py`
- [ ] Reference Anchor program: `tests/anchor-reference/pda_patterns/`
- [ ] Patterns: single seed, multiple seeds, user-derived PDAs, bump handling
- [ ] Tests verify correct PDA addresses are derived
- [ ] Tests verify bump seeds are stored/used correctly
- [ ] Behavior parity verified

#### US-014: Account initialization patterns
**Description:** As a user, I want examples of different account initialization approaches.

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/init_patterns.py`
- [ ] Reference Anchor program: `tests/anchor-reference/init_patterns/`
- [ ] Patterns: init, init_if_needed, realloc, zero-copy accounts
- [ ] Tests verify space calculation matches
- [ ] Tests verify rent exemption handling
- [ ] Behavior parity verified

---

### Phase 3: Token Programs (SPL Integration)

#### US-015: Token vault (deposit/withdraw)
**Description:** As a user, I want a token vault example demonstrating SPL Token integration.

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/token_vault.py`
- [ ] Reference Anchor program: `tests/anchor-reference/token_vault/`
- [ ] Instructions: initialize_vault, deposit, withdraw
- [ ] CPI to Token Program for transfers
- [ ] PDA-owned token account
- [ ] Tests verify token balances change correctly
- [ ] Behavior parity verified

#### US-016: Token minting program
**Description:** As a user, I want an example demonstrating mint authority and token minting.

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/token_mint.py`
- [ ] Reference Anchor program: `tests/anchor-reference/token_mint/`
- [ ] Instructions: create_mint, mint_tokens, burn_tokens
- [ ] PDA as mint authority
- [ ] Tests verify supply changes correctly
- [ ] Behavior parity verified

#### US-017: Associated Token Account handling
**Description:** As a user, I want examples showing ATA creation and usage patterns.

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/ata_patterns.py`
- [ ] Reference Anchor program: `tests/anchor-reference/ata_patterns/`
- [ ] Patterns: create ATA, init_if_needed for ATA, transfer to ATA
- [ ] CPI to Associated Token Program
- [ ] Tests verify ATA addresses match expected derivation
- [ ] Behavior parity verified

#### US-018: Token-2022 extensions
**Description:** As a user, I want examples using Token-2022 program features.

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/token_2022.py`
- [ ] Reference Anchor program: `tests/anchor-reference/token_2022/`
- [ ] Features: transfer fees, interest-bearing, metadata pointer
- [ ] CPI to Token-2022 Program
- [ ] Tests verify extension data is set correctly
- [ ] Behavior parity verified

---

### Phase 4: DeFi Patterns

#### US-019: Escrow program - implementation
**Description:** As a user, I want a complete escrow example for trustless token swaps.

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/escrow.py`
- [ ] Reference Anchor program: `tests/anchor-reference/escrow/`
- [ ] Instructions: initialize_escrow, accept_escrow, cancel_escrow
- [ ] Handles two-party token exchange
- [ ] Typecheck passes

#### US-020: Escrow program - tests
**Description:** As a developer, I need comprehensive tests for the escrow program.

**Acceptance Criteria:**
- [ ] Tests full happy path (init -> accept)
- [ ] Tests cancellation flow
- [ ] Tests failure cases (wrong amounts, wrong parties)
- [ ] Behavior parity verified

#### US-021: Token swap (AMM basics) - implementation
**Description:** As a user, I want a basic AMM/swap example demonstrating liquidity pool mechanics.

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/token_swap.py`
- [ ] Reference Anchor program: `tests/anchor-reference/token_swap/`
- [ ] Instructions: initialize_pool, add_liquidity, remove_liquidity, swap
- [ ] Constant product formula (x * y = k)
- [ ] LP token minting/burning
- [ ] Typecheck passes

#### US-022: Token swap (AMM basics) - tests
**Description:** As a developer, I need comprehensive tests for the AMM program.

**Acceptance Criteria:**
- [ ] Tests verify swap math is correct
- [ ] Tests verify LP token accounting
- [ ] Tests verify slippage handling
- [ ] Behavior parity verified

#### US-023: Staking program
**Description:** As a user, I want a token staking example with rewards calculation.

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/staking.py`
- [ ] Reference Anchor program: `tests/anchor-reference/staking/`
- [ ] Instructions: initialize_pool, stake, unstake, claim_rewards
- [ ] Time-based reward calculation
- [ ] Tests use LiteSVM time warping for reward accumulation
- [ ] Tests verify reward math
- [ ] Behavior parity verified

#### US-024: Lending pool (deposit/borrow)
**Description:** As a user, I want a simplified lending example demonstrating collateralization.

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/lending.py`
- [ ] Reference Anchor program: `tests/anchor-reference/lending/`
- [ ] Instructions: deposit_collateral, borrow, repay, withdraw_collateral
- [ ] Collateral ratio enforcement
- [ ] Interest accrual (simplified)
- [ ] Tests verify collateral requirements
- [ ] Tests verify interest calculation
- [ ] Behavior parity verified

#### US-025: Vesting schedule
**Description:** As a user, I want a token vesting example with cliff and linear release.

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/vesting.py`
- [ ] Reference Anchor program: `tests/anchor-reference/vesting/`
- [ ] Instructions: create_vesting, claim_vested
- [ ] Cliff period, linear vesting after cliff
- [ ] Tests use LiteSVM time warping
- [ ] Tests verify partial claims work correctly
- [ ] Behavior parity verified

#### US-026: Auction program
**Description:** As a user, I want an auction example demonstrating time-based state transitions.

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/auction.py`
- [ ] Reference Anchor program: `tests/anchor-reference/auction/`
- [ ] Instructions: create_auction, place_bid, end_auction, claim_prize
- [ ] Highest bidder tracking, bid refunds
- [ ] Time-based end condition
- [ ] Tests use LiteSVM time warping for auction end
- [ ] Behavior parity verified

---

### Phase 5: NFT Patterns

#### US-027: NFT minting with Metaplex
**Description:** As a user, I want an NFT minting example using Metaplex Token Metadata.

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/nft_mint.py`
- [ ] Reference Anchor program: `tests/anchor-reference/nft_mint/`
- [ ] CPI to Metaplex Token Metadata program
- [ ] Create metadata, mint NFT, set URI
- [ ] Tests verify metadata account created correctly
- [ ] Behavior parity verified

#### US-028: NFT collection with verification
**Description:** As a user, I want an NFT collection example with verified collection membership.

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/nft_collection.py`
- [ ] Reference Anchor program: `tests/anchor-reference/nft_collection/`
- [ ] Create collection NFT, mint to collection, verify
- [ ] Collection authority handling
- [ ] Tests verify collection verification status
- [ ] Behavior parity verified

#### US-029: NFT staking - implementation
**Description:** As a user, I want an NFT staking example with reward tokens.

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/nft_staking.py`
- [ ] Reference Anchor program: `tests/anchor-reference/nft_staking/`
- [ ] Instructions: stake_nft, unstake_nft, claim_rewards
- [ ] NFT custody (transfer to PDA)
- [ ] Time-based reward calculation
- [ ] Typecheck passes

#### US-030: NFT staking - tests
**Description:** As a developer, I need comprehensive tests for the NFT staking program.

**Acceptance Criteria:**
- [ ] Tests verify NFT ownership transfers correctly
- [ ] Tests verify reward accumulation
- [ ] Tests verify unstaking returns NFT
- [ ] Behavior parity verified

#### US-031: NFT marketplace - implementation
**Description:** As a user, I want an NFT marketplace example for listing and purchasing.

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/nft_marketplace.py`
- [ ] Reference Anchor program: `tests/anchor-reference/nft_marketplace/`
- [ ] Instructions: list_nft, buy_nft, delist_nft
- [ ] Escrow NFT during listing
- [ ] Price in SOL or SPL token
- [ ] Royalty handling (optional)
- [ ] Typecheck passes

#### US-032: NFT marketplace - tests
**Description:** As a developer, I need comprehensive tests for the NFT marketplace program.

**Acceptance Criteria:**
- [ ] Tests verify listing state
- [ ] Tests verify purchase transfers NFT and payment
- [ ] Tests verify delisting returns NFT
- [ ] Behavior parity verified

---

### Phase 6: Governance & DAO Patterns

#### US-033: Simple voting
**Description:** As a user, I want a basic voting example demonstrating proposal and vote mechanics.

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/voting.py`
- [ ] Reference Anchor program: `tests/anchor-reference/voting/`
- [ ] Instructions: create_proposal, cast_vote, finalize_proposal
- [ ] One vote per user (voter record PDA)
- [ ] Vote counting
- [ ] Tests verify vote tallying
- [ ] Tests verify double-vote prevention
- [ ] Behavior parity verified

#### US-034: Token-weighted governance
**Description:** As a user, I want a governance example where voting power is proportional to tokens held.

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/governance.py`
- [ ] Reference Anchor program: `tests/anchor-reference/governance/`
- [ ] Voting power based on token balance at snapshot
- [ ] Quorum requirements
- [ ] Timelock execution
- [ ] Tests verify weighted vote calculation
- [ ] Behavior parity verified

#### US-035: Multisig wallet
**Description:** As a user, I want a multisig example requiring M-of-N signatures.

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/multisig.py`
- [ ] Reference Anchor program: `tests/anchor-reference/multisig/`
- [ ] Instructions: create_multisig, propose_transaction, approve, execute
- [ ] Configurable threshold (M of N)
- [ ] Tests verify threshold enforcement
- [ ] Tests verify transaction execution after threshold met
- [ ] Behavior parity verified

---

### Phase 7: Advanced Patterns

#### US-036: Oracle price feed consumer
**Description:** As a user, I want an example consuming oracle price feeds (Pyth/Switchboard pattern).

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/oracle_consumer.py`
- [ ] Reference Anchor program: `tests/anchor-reference/oracle_consumer/`
- [ ] Read price from oracle account
- [ ] Staleness check
- [ ] Confidence interval handling
- [ ] Tests mock oracle account data
- [ ] Behavior parity verified

#### US-037: Cross-program invocation patterns
**Description:** As a user, I want examples demonstrating various CPI patterns.

**Acceptance Criteria:**
- [x] Seahorse program: `examples/cpi_patterns.py`
- [x] Reference Anchor program: `tests/anchor-reference/cpi_patterns_anchor/`
- [x] Patterns: basic CPI, CPI with signer seeds, CPI with remaining accounts
- [x] Error handling across CPI boundary
- [x] Tests verify CPI executes correctly
- [x] Behavior parity verified

#### US-038: Realloc and close patterns
**Description:** As a user, I want examples for dynamic account resizing and closing.

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/realloc_close.py`
- [ ] Reference Anchor program: `tests/anchor-reference/realloc_close/`
- [ ] Instructions: grow_account, shrink_account, close_account
- [ ] Rent handling on realloc
- [ ] Lamport refund on close
- [ ] Tests verify space changes correctly
- [ ] Tests verify lamports returned on close
- [ ] Behavior parity verified

#### US-039: Event emission patterns
**Description:** As a user, I want examples demonstrating Anchor event emission.

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/events.py`
- [ ] Reference Anchor program: `tests/anchor-reference/events/`
- [ ] Emit events with various data types
- [ ] Tests capture and verify emitted events
- [ ] TypeScript tests parse events from transaction logs
- [ ] Behavior parity verified

#### US-040: Error handling patterns
**Description:** As a user, I want examples demonstrating custom error definitions and handling.

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/errors.py`
- [ ] Reference Anchor program: `tests/anchor-reference/errors/`
- [ ] Custom error enum with codes
- [ ] require! macro usage patterns
- [ ] Tests verify correct error codes returned
- [ ] Tests verify error messages (if accessible)
- [ ] Behavior parity verified

#### US-041: Access control patterns
**Description:** As a user, I want examples of various access control mechanisms.

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/access_control.py`
- [ ] Reference Anchor program: `tests/anchor-reference/access_control/`
- [ ] Patterns: owner-only, admin roles, whitelist, time-locked
- [ ] Tests verify unauthorized access is rejected
- [ ] Tests verify authorized access succeeds
- [ ] Behavior parity verified

#### US-042: Batch operations
**Description:** As a user, I want examples processing multiple items in a single instruction.

**Acceptance Criteria:**
- [ ] Seahorse program: `examples/batch_ops.py`
- [ ] Reference Anchor program: `tests/anchor-reference/batch_ops/`
- [ ] Process vector of inputs
- [ ] Remaining accounts pattern for variable accounts
- [ ] Tests verify batch processing works correctly
- [ ] Tests verify compute budget handling
- [ ] Behavior parity verified

---

### Phase 8: Test Coverage & CI

#### US-043: CI pipeline for test suite
**Description:** As a maintainer, I need a CI pipeline that runs all tests automatically.

**Acceptance Criteria:**
- [ ] GitHub Actions workflow for test suite
- [ ] Parallel test execution where possible
- [ ] Rust tests (Mollusk + LiteSVM)
- [ ] TypeScript tests
- [ ] Test result reporting
- [ ] Failure notifications

#### US-044: CU tracking for informational analysis
**Description:** As a maintainer, I want to track compute unit usage across examples for analysis and comparison.

**Acceptance Criteria:**
- [ ] Compute unit tracking using Mollusk bencher
- [ ] Record CU usage for all Seahorse examples
- [ ] Record CU usage for reference Anchor programs
- [ ] Generate comparison report (Seahorse vs Anchor CU)
- [ ] Historical tracking of CU trends (no pass/fail thresholds)

---

## Functional Requirements

- FR-1: **Solana SDK 3.0 compatibility is the prerequisite for all other work**
- FR-2: Test infrastructure must support both Mollusk (unit) and LiteSVM (integration) harnesses
- FR-3: Example programs are derived from existing idiomatic Anchor programs (find Anchor first, write Seahorse to match)
- FR-4: Behavior parity tests must compare account state after identical instruction sequences
- FR-5: Tests must support time manipulation for time-dependent logic (via LiteSVM warp_to_slot)
- FR-6: CPI tests must work with SPL Token, Associated Token, and Metaplex programs
- FR-7: TypeScript tests must use anchor-litesvm for Anchor IDL integration
- FR-8: Test utilities must handle PDA derivation, account creation, and transaction building
- FR-9: All tests must be runnable via single command (`make test` or equivalent)
- FR-10: Tests must produce clear output indicating which comparisons passed/failed
- FR-11: Example programs must follow Seahorse best practices and idioms

## Non-Goals

- Supporting Solana SDK versions below 3.0
- Fuzzing infrastructure (separate initiative)
- Performance benchmarking against other languages (only CU tracking)
- Mainnet deployment testing (use devnet/localnet only)
- UI/frontend testing
- Multi-language Seahorse examples (Python only)
- Formal verification
- Backwards compatibility with older Anchor versions

## Technical Considerations

### SDK 3.0 Upgrade Strategy
1. Update core solana-sdk/solana-program dependencies first
2. Update Anchor to version supporting SDK 3.0
3. Update test harnesses (mollusk-svm, litesvm)
4. Verify generated code compiles with new dependencies
5. Run full test suite to validate

### Testing Stack
- **Mollusk** (`mollusk-svm`): Isolated instruction unit tests
- **LiteSVM** (`litesvm`, `anchor-litesvm`): Integration tests with full transaction flow
- **TypeScript**: `@coral-xyz/anchor`, `anchor-litesvm` npm packages
- **Build**: Anchor CLI for reference programs, Seahorse CLI for examples

### Directory Structure
```
tests/
├── unit/                    # Mollusk unit tests (Rust)
│   └── *.rs
├── integration/             # LiteSVM integration tests (Rust)
│   └── *.rs
├── ts/                      # TypeScript tests
│   ├── package.json
│   └── *.test.ts
├── anchor-reference/        # Reference Anchor programs
│   ├── counter/
│   ├── escrow/
│   └── ...
├── fixtures/                # Test data, account snapshots
└── README.md
examples/
├── counter.py
├── escrow.py
└── ...                      # 30+ Seahorse examples
```

### Key Dependencies
- `solana-sdk` 3.0.x - Core SDK
- `solana-program` 3.0.x - Program development
- `anchor-lang` 0.30+ - Anchor framework (SDK 3.0 compatible)
- `mollusk-svm` - Anza's SVM test harness (SDK 3.0 version)
- `litesvm` - Fast SVM for Rust (SDK 3.0 version)
- `anchor-litesvm` - Anchor + LiteSVM integration
- `spl-token`, `spl-associated-token-account` - Token program testing
- `mpl-token-metadata` - Metaplex integration testing

### CPI Testing Strategy
- Mock program accounts where possible for speed
- Use LiteSVM's ability to load real program binaries for critical CPI tests
- Consider Surfpool for complex CPI scenarios (Jupiter-like)

## Success Metrics

- Solana SDK 3.0 upgrade complete with all existing tests passing
- 30+ example programs with passing tests
- 100% of examples have behavior parity tests
- <5 second average test execution time (Mollusk unit tests)
- <30 second full integration test suite
- Zero false positives in behavior comparison
- CI runs complete in <10 minutes
- CU usage tracked and recorded for all examples (informational, no pass/fail threshold)

## Resolved Questions

1. **SDK version** - Target Solana SDK 3.0.x as the baseline

2. **CU comparison** - Track compute unit usage for informational purposes only. No pass/fail thresholds; just record for analysis.

3. **Seahorse-only features** - Test in isolation. These don't need Anchor reference comparisons.

4. **Reference program approach** - Find existing idiomatic Anchor programs first, then write Seahorse versions to match them. Not the reverse.

5. **Solana version targeting** - Target latest Solana/Anchor versions only. No multi-version compatibility matrix.

6. **TypeScript test priority** - TypeScript tests are secondary to Rust tests. Rust (Mollusk + LiteSVM) is the primary test harness.
