# Seahorse Constraint Parity - Shared Learnings

This document contains important discoveries, patterns, and notes that all agents should be aware of during implementation.

---

## Codebase Structure

### Key Files
- `src/core/compile/ast.rs` - Contains `AccountAnnotation`, `Transformed` enum, expression types
- `src/core/compile/build/mod.rs` - Build stage, `ExprContext` enum, constraint merging
- `src/core/compile/check/mod.rs` - Typecheck hooks, method transformations
- `src/core/generate/mod.rs` - Codegen, `AccountAnnotationWithTyExpr::to_tokens()`
- `src/core/compile/builtin/prelude.rs` - Prelude type definitions and transformations
- `data/const/seahorse_prelude.py` - Python prelude stubs for IDE support

### Pipeline Flow
1. Parse Python → Clean AST
2. Preprocess/import-resolve
3. Typecheck + record transformations
4. Build lower IR (statements + account annotations)
5. Generate Anchor Rust

---

## Important Patterns

### ExprContext Usage
The `ExprContext` enum controls how expressions are lowered:
- `LVal` - Left-value (assignment target)
- `Seed` - Seeds context (no borrow injection, special casts)
- `Directive` - Directive context
- `Assert` - Assert context
- **`AccountAttr`** (NEW) - Anchor validation scope expressions

### Account Transformations
Constraints are attached via method calls that return `Transformed::AccountXxx` variants:
- `.has_one(target)` → `Transformed::AccountHasOne`
- `.close(recipient)` → `Transformed::AccountClose`
- `.realloc(...)` → `Transformed::AccountRealloc`

### Type System Notes
- `Ty::Anonymous(0)` is the "account base type" used for constraint method lookups
- `ty.is_mut()` returns true for most account types (this is the "everything is mut" problem)

---

## Gotchas & Warnings

### 1. Transformation Equality Hack
`Transformation` implements `PartialEq` as always-equal. Don't rely on equality comparisons.

### 2. Empty.bump - FIXED
Previously `Empty` accounts unconditionally got `bump: Some(ctx.bumps.name)` even without seeds.
**FIXED in `src/core/generate/mod.rs`**: Now checks `annotation.seeds` before setting bump.
- If seeds are present: `bump: Some(ctx.bumps.name)`
- If no seeds: `bump: None`

### 3. Borrow Injection
Build stage injects `.borrow()`/`.borrow_mut()` based on `ty.is_mut()`. This must be disabled for `AccountAttr` context.

### 4. key() Rewriting
In handler scope, `.key()` gets rewritten to `.borrow().__account__.key()`. Must be disabled for `AccountAttr`.

---

## Anchor Constraint Reference

### Core Constraints (from docs)
- `mut`, `signer`, `dup` - flags with optional `@ error`
- `init`, `init_if_needed` - initialization
- `seeds = [...]`, `bump`, `bump = <expr>`, `seeds::program` - PDA
- `has_one`, `address`, `owner`, `executable`, `zero`, `close`
- `constraint = <expr>` - arbitrary expressions
- `realloc`, `realloc::payer`, `realloc::zero`
- `rent_exempt = skip|enforce`

### SPL Constraints
- `token::mint`, `token::authority`, `token::token_program`
- `mint::decimals`, `mint::authority`, `mint::freeze_authority`, `mint::token_program`
- `associated_token::mint`, `associated_token::authority`, `associated_token::token_program`

---

## Implementation Notes

### ExprContext::AccountAttr Implementation (2026-01-22)

Added the `AccountAttr` variant to handle Anchor constraint attribute expressions that run in the `derive(Accounts)` validation scope where variables are raw Anchor account fields.

#### Files Modified

1. **`src/core/compile/build/mod.rs`**
   - Added `AccountAttr` to `ExprContext` enum (line 91)
   - Updated Index borrow injection guard (line 637): `if value.ty.is_mut() && !context_stack.has(&ExprContext::AccountAttr)`
   - Updated Attribute borrow injection guard (line 668): same pattern
   - Updated list literal Mutable wrapping (lines 771-775): added AccountAttr to exemption list
   - Updated comprehension Mutable wrapping (lines 869-874): added AccountAttr to exemption list
   - Updated string literal `.to_string()` conversion (lines 876-882): added AccountAttr to exemption list

2. **`src/core/compile/ast.rs`**
   - Updated `TypedExpression::moved()` (lines 321-327): added AccountAttr to contexts that skip Move wrapping

3. **`src/core/compile/check/mod.rs`**
   - Updated `.key()` rewriting guard for defined accounts (line 820):
     - Changed: `!context_stack.has(&ExprContext::Seed)`
     - To: `!context_stack.has_any(&[ExprContext::Seed, ExprContext::AccountAttr])`

4. **`src/core/compile/builtin/prelude.rs`**
   - Updated all Seed-type transformations to also strip borrows for AccountAttr:
     - RustInt::Seed (line 1652)
     - Signer::Seed (line 1709)
     - Pubkey::Seed (line 1730)
     - Program::Seed (line 1755)
     - TokenMint::Seed (line 1779)
     - TokenAccount::Seed (line 1803)

#### Key Pattern
AccountAttr context follows the same patterns as Seed context:
- Skip Move wrapping
- Skip borrow injection
- Skip Mutable wrapping for collections
- Keep strings as raw literals
- Skip `.borrow().__account__.key()` rewriting

