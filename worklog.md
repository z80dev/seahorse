# Constraint Parity Implementation Worklog

## Overview
Implementing constraint parity between Seahorse and Anchor as outlined in `constraint_parity.md`.

## Status: In Progress

---

## Phase 1: Foundation Work

### Task 1.1: Add ExprContext::AccountAttr
- **Status**: COMPLETED ✓
- **Agent**: Codex Agent #1 (affa247)
- **Files**: `src/core/compile/build/mod.rs`, `src/core/compile/ast.rs`, `src/core/compile/check/mod.rs`, `src/core/compile/builtin/prelude.rs`
- **Notes**: Added AccountAttr variant, updated all borrow injection guards, key() rewriting, Move wrapping, and literal handling

### Task 1.2: Refactor AccountConstraint enum
- **Status**: Pending
- **Agent**: TBD
- **Files**: `src/core/compile/ast.rs`
- **Notes**: Replace special transformed variants with generic constraint system

### Task 1.3: Fix Empty.bump correctness
- **Status**: COMPLETED ✓
- **Agent**: Codex Agent #2 (a334eec)
- **Files**: `src/core/generate/mod.rs`
- **Notes**: Now conditionally sets bump based on whether seeds are present

---

## Phase 2: Core Constraints Implementation

### Task 2.1: Implement `address` constraint
- **Status**: Pending

### Task 2.2: Implement `owner` constraint
- **Status**: Pending

### Task 2.3: Implement `signer` constraint
- **Status**: Pending

### Task 2.4: Implement `dup` constraint
- **Status**: Pending

### Task 2.5: Implement `executable` constraint
- **Status**: Pending

### Task 2.6: Implement `zero` constraint
- **Status**: Pending

### Task 2.7: Implement `constraint = <expr>`
- **Status**: Pending

### Task 2.8: Implement `rent_exempt`
- **Status**: Pending

### Task 2.9: Implement `seeds::program`
- **Status**: Pending

### Task 2.10: Implement `bump = <expr>`
- **Status**: Pending

### Task 2.11: Implement seeds on non-init accounts
- **Status**: Pending

---

## Phase 3: SPL Constraints

### Task 3.1: SPL constraints as checks (not just init)
- **Status**: Pending

### Task 3.2: token_program override constraints
- **Status**: Pending

---

## Phase 4: Mutability Fixes

### Task 4.1: Implement `readonly()` method
- **Status**: Pending

### Task 4.2: Stop emitting `mut` everywhere
- **Status**: Pending

---

## Commits Log

| Date | Commit | Description |
|------|--------|-------------|
| 2026-01-22 | 6b01a1e | Phase 1: ExprContext::AccountAttr + Empty.bump fix |
| 2026-01-22 | 6e8b5d7 | Core constraints: address, owner, executable |
| 2026-01-22 | 9a07641 | Flag constraints: signer, dup, zero |
| 2026-01-22 | e6aa8c6 | Advanced constraints: constraint=expr, rent_exempt, seeds::program |

---

## Review Log

| Date | Reviewer | Target | Result |
|------|----------|--------|--------|

