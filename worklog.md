# Constraint Parity Implementation Worklog

## Overview
Implementing constraint parity between Seahorse and Anchor as outlined in `constraint_parity.md`.

## Status: COMPLETED ✓

---

## Phase 1: Foundation Work

### Task 1.1: Add ExprContext::AccountAttr
- **Status**: COMPLETED ✓
- **Agent**: Codex Agent (affa247)
- **Files**: `src/core/compile/build/mod.rs`, `src/core/compile/ast.rs`, `src/core/compile/check/mod.rs`, `src/core/compile/builtin/prelude.rs`
- **Notes**: Added AccountAttr variant, updated all borrow injection guards, key() rewriting, Move wrapping, and literal handling

### Task 1.2: Fix Empty.bump correctness
- **Status**: COMPLETED ✓
- **Agent**: Codex Agent (a334eec)
- **Files**: `src/core/generate/mod.rs`
- **Notes**: Now conditionally sets bump based on whether seeds are present

---

## Phase 2: Core Constraints Implementation

### Task 2.1: Implement `address` constraint
- **Status**: COMPLETED ✓
- **Agent**: Codex Agent (ad9361e)

### Task 2.2: Implement `owner` constraint
- **Status**: COMPLETED ✓
- **Agent**: Codex Agent (a58c1c1)

### Task 2.3: Implement `executable` constraint
- **Status**: COMPLETED ✓
- **Agent**: Codex Agent (ad24d0b)

### Task 2.4: Implement `signer` constraint
- **Status**: COMPLETED ✓
- **Agent**: Codex Agent (adbff0b)

### Task 2.5: Implement `dup` constraint
- **Status**: COMPLETED ✓
- **Agent**: Codex Agent (a5a4beb)

### Task 2.6: Implement `zero` constraint
- **Status**: COMPLETED ✓
- **Agent**: Codex Agent (a5324d9)

### Task 2.7: Implement `constraint = <expr>`
- **Status**: COMPLETED ✓
- **Agent**: Codex Agent (abe45b4)
- **Notes**: Critical - uses AccountAttr context for proper expression generation

### Task 2.8: Implement `rent_exempt`
- **Status**: COMPLETED ✓
- **Agent**: Codex Agent (ad1ae7c)
- **Notes**: Added RentExemptMode enum (Skip/Enforce)

### Task 2.9: Implement `seeds::program`
- **Status**: COMPLETED ✓
- **Agent**: Codex Agent (a9ba3c0)

### Task 2.10: Implement `bump = <expr>`
- **Status**: COMPLETED ✓
- **Agent**: Codex Agent (a02990e)

### Task 2.11: Implement seeds on non-init accounts
- **Status**: COMPLETED ✓
- **Agent**: Codex Agent (afb8844)

---

## Phase 3: SPL Constraints

### Task 3.1: SPL constraints as checks (not just init)
- **Status**: COMPLETED ✓
- **Agent**: Codex Agent (a2746de)
- **Notes**: Added token_mint(), token_authority(), token_program(), mint_decimals(), mint_authority(), mint_freeze_authority(), mint_token_program()

---

## Phase 4: Mutability Fixes

### Task 4.1: Implement `readonly()` method
- **Status**: COMPLETED ✓
- **Agent**: Codex Agent (aa6ee66)
- **Notes**: Fixes "all accounts are mut" problem

---

## Summary of New Constraints Implemented

### Core Constraints
| Constraint | Seahorse API | Anchor Output |
|------------|--------------|---------------|
| address | `account.address(pubkey)` | `#[account(address = pubkey)]` |
| owner | `account.owner(pubkey)` | `#[account(owner = pubkey)]` |
| executable | `account.executable()` | `#[account(executable)]` |
| signer | `account.signer()` | `#[account(signer)]` |
| dup | `account.dup()` | `#[account(dup)]` |
| zero | `account.zero()` | `#[account(zero)]` |
| constraint | `account.constraint(expr)` | `#[account(constraint = expr)]` |
| rent_exempt | `account.rent_exempt("skip")` | `#[account(rent_exempt = skip)]` |
| seeds::program | `account.seeds_program(prog)` | `#[account(seeds::program = prog)]` |
| bump=expr | `account.bump(value)` | `#[account(bump = value)]` |
| seeds | `account.seeds([...])` | `#[account(seeds = [...], bump)]` |
| readonly | `account.readonly()` | Omits `mut` constraint |

### SPL Constraints
| Constraint | Seahorse API | Anchor Output |
|------------|--------------|---------------|
| token::mint | `token.token_mint(mint)` | `#[account(token::mint = mint)]` |
| token::authority | `token.token_authority(auth)` | `#[account(token::authority = auth)]` |
| token::token_program | `token.token_program(prog)` | `#[account(token::token_program = prog)]` |
| mint::decimals | `mint.mint_decimals(9)` | `#[account(mint::decimals = 9)]` |
| mint::authority | `mint.mint_authority(auth)` | `#[account(mint::authority = auth)]` |
| mint::freeze_authority | `mint.mint_freeze_authority(auth)` | `#[account(mint::freeze_authority = auth)]` |
| mint::token_program | `mint.mint_token_program(prog)` | `#[account(mint::token_program = prog)]` |

---

## Commits Log

| Date | Commit | Description |
|------|--------|-------------|
| 2026-01-22 | 6b01a1e | Phase 1: ExprContext::AccountAttr + Empty.bump fix |
| 2026-01-22 | 6e8b5d7 | Core constraints: address, owner, executable |
| 2026-01-22 | 9a07641 | Flag constraints: signer, dup, zero |
| 2026-01-22 | e6aa8c6 | Advanced constraints: constraint=expr, rent_exempt, seeds::program |
| 2026-01-22 | 59cd24b | PDA constraints: bump=expr, seeds on non-init |
| 2026-01-22 | 99e78fc | SPL constraints + readonly method |

---

## Review Log

| Date | Reviewer | Target | Result |
|------|----------|--------|--------|
| 2026-01-22 | Codex Agent (a55dd79) | Phase 1 changes | PASSED - all items verified correct |

---

## Build Verification

- **cargo build**: SUCCESS (warnings are pre-existing, unrelated to changes)
- **cargo test**: SUCCESS (all tests pass)

