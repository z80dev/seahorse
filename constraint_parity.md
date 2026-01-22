Below is a **constraint‑parity implementation plan** that’s grounded in how Seahorse currently works (transformations collected during typecheck → merged into `AccountAnnotation` during build → emitted into `#[account(...)]` during generate), and mapped against **Anchor’s documented constraint surface**.

I’m going to be very explicit about:

* **What Anchor supports** (by category)
* **What Seahorse currently supports**
* **Exactly what needs to change** in the compiler (data model, typechecker hooks, build merge, codegen)
* The two “hard parts” you must solve to get this “perfect”:

  1. **Constraint-expression lowering** (Anchor-scope expressions vs Seahorse loaded/mutable wrappers)
  2. **`mut`/`dup` parity** (and why Seahorse’s current “everything is mut” is a big practical mismatch)

---

## 1) Anchor constraint surface (source of truth)

### Core account constraints (anchor-lang / derive)

Anchor’s “core” constraints include (non-exhaustive list shown in docs):
`mut`, `signer`, `dup`, `init`, `init_if_needed`, `seeds` + `bump` (including `bump = <expr>`), `seeds::program`, `has_one`, `address`, `owner`, `executable`, `zero`, `close`, `constraint`, `realloc`. ([Anchor Lang][1])

Also, `rent_exempt = skip|enforce` exists (docs.rs for anchor derive accounts) and is referenced as something `init`/`zero` enforce unless skipped. ([Docs.rs][2])

Key semantics that affect Seahorse design:

* `dup` is specifically about allowing **duplicate mutable accounts**; read-only duplicates are naturally allowed. ([Anchor Lang][1])
* `init_if_needed` requires enabling a feature flag. ([Anchor Lang][1])
* `seeds::program` has restrictions (docs.rs: cannot be used with `init`). ([Docs.rs][2])

### SPL constraints (token, mint, ATA)

Anchor’s SPL constraints include:

* `token::mint`, `token::authority`, and `token::token_program`
* `mint::decimals`, `mint::authority`, `mint::freeze_authority`, and `mint::token_program`
* `associated_token::mint`, `associated_token::authority`, and `associated_token::token_program`
  and they explicitly note that these constraints can be used as **checks** (not only init), and that partial subsets can be specified. ([Docs.rs][2])

### Token extensions constraints (Token-2022)

Anchor documents a set of `extensions::...` constraints for Token-2022 extensions (close authority, permanent delegate, transfer hook program id, group pointers, metadata pointers, etc.). ([Anchor Lang][1])

---

## 2) Where Seahorse is today (constraints)

From the codebase you provided, Seahorse currently emits these constraint shapes:

**Already supported (core):**

* `init`, `init_if_needed`, `payer`, `space`, `seeds = [...]`, and implicitly `bump` (only the “bare bump” form)
* `close = <ident>`
* `realloc = <expr>`, `realloc::payer = <ident>`, `realloc::zero = <expr>`
* `has_one = <ident>` (vector)
* `mut` is effectively “always on” for non-init accounts because `AccountAnnotation.is_mut` is set from `ty.is_mut()`, and account-ish tys always return `true`.

**Already supported (SPL init-only):**

* `mint::decimals`, `mint::authority`
* `token::mint`, `token::authority`
* `associated_token::mint`, `associated_token::authority`

**Missing (core parity gaps):**

* `signer` (as a constraint)
* `dup`
* `address`
* `owner`
* `executable`
* `zero`
* `constraint = <expr>`
* `rent_exempt = skip|enforce`
* `seeds::program`
* `bump = <expr>` form
* seeds/constraints for **non-init PDAs** (right now `seeds` only exists via `Empty.init`)

**Missing (SPL parity gaps):**

* `*::token_program` constraints (`token::token_program`, `mint::token_program`, `associated_token::token_program`) ([Docs.rs][2])
* SPL constraints used as checks (i.e., set token/mint/ata constraints without `init`)

**Missing (Token-2022 extensions):**

* All `extensions::...` constraints ([Anchor Lang][1])

---

## 3) Constraint parity plan (deep implementation)

### Guiding principle

Stop thinking of constraints as “special cases” (`AccountInit`, `AccountHasOne`, `AccountClose`, …) and move toward:

* A **single constraint pipeline**: “collect constraints → merge/validate → emit”
* A **single expression lowering mode** for “Anchor-scope expressions” (the expressions inside `#[account(...)]`), which is *not the same* as Seahorse’s loaded/mutable runtime view.

This is the core unlock for `constraint = ...`, `address = ...`, `owner = ...`, `bump = ...`, token_program overrides, etc.

---

## 4) Step 1: Introduce Anchor-scope expression mode

### Why you need this

Right now Seahorse builds expressions assuming handler-scope “loaded mutable wrappers”:

* For `.key()` in handler scope, Seahorse rewrites to `obj.borrow().__account__.key()` unless you’re in `ExprContext::Seed`.
* For attribute/index operations, build inserts `.borrow()`/`.borrow_mut()` based on `ty.is_mut()`.

That is correct for handler code, but wrong for **account-validation constraints**, because those constraints run in the `derive(Accounts)` generated validation where variables are the **raw Anchor accounts fields** (e.g., `Account<'info, T>`, `Signer<'info>`, etc.), not Seahorse’s runtime wrapper types.

If you try to implement `#[account(constraint = some_account.authority == signer.key())]` by naively reusing handler-scope lowering, you’ll emit things like `.borrow()` which simply do not exist in the generated validation scope.

### Concrete change

Add a new expression context, e.g.:

```rust
// src/core/compile/build/mod.rs
pub enum ExprContext {
    LVal,
    Seed,
    Directive,
    Assert,
    AccountAttr, // NEW: "Anchor validation scope"
}
```

Then update all “special lowering” points to treat `AccountAttr` like (or similar to) `Seed`, with a few differences:

#### 4.1 `TypedExpression::moved` must not insert `Move(...)` in AccountAttr

```rust
// src/core/compile/ast.rs
pub fn moved(mut self, context_stack: &ExprContextStack) -> Self {
    if !context_stack.has_any(&[
        ExprContext::Directive,
        ExprContext::Seed,
        ExprContext::AccountAttr, // NEW
    ]) {
        self.obj = ExpressionObj::Move(self.obj.into());
    }
    self
}
```

Reason: Anchor attribute constraints should reference in-scope vars (identifiers), not `clone()`d temporaries.

#### 4.2 Disable `.borrow()` injection in AccountAttr

In build’s `Attribute`/`Index` lowering:

```rust
// src/core/compile/build/mod.rs (in build_expression)
if ty.is_mut() && !context_stack.has(&ExprContext::AccountAttr) {
   // current borrow insertion logic
}
```

#### 4.3 Treat literals the way attribute constraints expect

Strings/lists currently get wrapped into owned `String` / `Mutable(vec![])` in non-seed contexts. In `AccountAttr`, you almost always want raw literals:

* `"abc"` should remain `"abc"` (not `"abc".to_string()` unless explicitly required)
* list literals should remain plain `vec![...]` or array literal or something anchor accepts

Reuse the Seed/Directive behavior for `Str`, list/lc, etc by including AccountAttr in those checks.

#### 4.4 Expand key() rewriting guard

In check stage, `.key()` rewrite currently only exempts Seed context. Expand it:

```rust
if !context_stack.has_any(&[ExprContext::Seed, ExprContext::AccountAttr]) {
    // rewrite to borrow().__account__.key()
}
```

This single change is critical if you want `constraint = payer.key() == authority.key()` to generate valid Anchor-scope code.

---

## 5) Step 2: Replace “special transformed variants per constraint” with a generic constraint patch

Right now you have:

```rust
pub enum Transformed {
  AccountInit { ... },
  AccountRealloc { ... },
  AccountClose { ... },
  AccountHasOne { ... },
  ...
}
```

This does not scale to Anchor parity (you’d add ~15 more variants just for core constraints, then SPL, then extensions).

### Replace with:

```rust
pub enum Transformed {
  Expression(TypedExpression),
  Cpi { ... },
  AccountConstraint {
      expr: TypedExpression,     // rewritten no-op expr or init-return expr
      name: String,              // account field name in ctx
      constraint: AccountConstraint,
  },
  Directive(Directive),
}
```

And define:

```rust
#[derive(Clone, Debug)]
pub enum AccountConstraint {
    Mut { error: Option<ConstraintError> },
    Signer { error: Option<ConstraintError> },
    Dup { error: Option<ConstraintError> },

    Init {
        payer: TypedExpression,
        space: Option<TypedExpression>, // required for user-defined accounts; optional in others
        seeds: Option<Vec<TypedExpression>>,
        bump: Option<BumpSpec>,         // bare bump or bump = expr
        owner: Option<TypedExpression>,
        rent_exempt: Option<RentExemptMode>,
        // plus token/mint/ata init-specific groups
        token: Option<TokenConstraintGroup>,
        mint: Option<MintConstraintGroup>,
        associated_token: Option<AssociatedTokenConstraintGroup>,
        error: Option<ConstraintError>,
    },

    Seeds {
        seeds: Vec<TypedExpression>,
        bump: BumpSpec,
        program: Option<TypedExpression>,  // seeds::program
        error: Option<ConstraintError>,
    },

    HasOne { target: String, error: Option<ConstraintError> },
    Address { expr: TypedExpression, error: Option<ConstraintError> },
    Owner   { expr: TypedExpression, error: Option<ConstraintError> },
    Executable { /* maybe allow error if Anchor supports */ },
    Zero { rent_exempt: Option<RentExemptMode> },

    Close { recipient: String },
    Constraint { expr: TypedExpression, error: Option<ConstraintError> },

    Realloc {
        space: TypedExpression,
        payer: String,
        zero: Option<TypedExpression>,
    },

    Token(TokenConstraintGroup),
    Mint(MintConstraintGroup),
    AssociatedToken(AssociatedTokenConstraintGroup),

    Extensions(ExtensionConstraints), // future-proof
}
```

Also add:

```rust
#[derive(Clone, Debug)]
pub enum BumpSpec {
    Implicit,                 // "bump"
    Explicit(TypedExpression) // "bump = <expr>"
}

#[derive(Clone, Debug)]
pub enum RentExemptMode { Enforce, Skip }
```

### Merge strategy

Change `AccountAnnotation` from “many fields” to:

```rust
pub struct AccountAnnotation {
    pub constraints: Vec<AccountConstraint>,
}
```

Then implement:

```rust
impl AccountAnnotation {
  pub fn push(&mut self, c: AccountConstraint) -> Result<(), Error>;
  pub fn validate(&self, account_ty: &AccountTyExpr) -> Result<(), Error>;
}
```

Where `push()`:

* rejects duplicates (`owner` twice, `address` twice, etc)
* enforces mutual exclusions (`init` with `realloc`, `init` with `zero`, etc)
* enforces Anchor rules you know statically (e.g., `dup` requires mut, `seeds::program` not with init, etc) ([Docs.rs][2])

This gives you one place to encode parity rules.

---

## 6) Step 3: Add Seahorse surface syntax for constraints

You can get parity without inventing a new language feature by reusing the existing “method call attaches constraint” pattern (like `.has_one`, `.realloc`, `.close`, `Empty.init`).

But you need to be careful with method naming collisions (TokenAccount already has `mint()`/`authority()` methods in the prelude stub).

### Proposed Seahorse API (Python)

#### Core constraints

```py
@instruction
def ix(
    payer: Signer,
    authority: Signer,
    pda: MyAccount,
    unchecked: UncheckedAccount,
    program: Program,
):
    # mut / readonly control (see mut-parity section below)
    authority.readonly()            # would omit `mut` on authority (and avoid dup issues)

    # signer constraint (useful when account isn't typed as Signer)
    pda.signer()

    # dup constraint
    authority.dup()

    # address / owner
    program.address(SOME_PROGRAM_ID)
    unchecked.owner(token_program.key())

    # executable / zero
    program.executable()
    pda.zero(rent_exempt="enforce")   # or skip

    # arbitrary constraint expression (Anchor-scope)
    pda.constraint(pda.authority == authority.key(), error="Unauthorized")
```

#### PDA seeds on non-init accounts

```py
pda.seeds(
    [b"pda", authority.key()],
    bump=bump_arg,                 # bump = <expr> (optional)
    program=other_program.key(),   # seeds::program (optional)
    error="BadPda",
)
```

This is required for parity because Anchor allows seeds constraints independent of init. ([Anchor Lang][1])

#### SPL constraints as checks (not init)

```py
token_acc.token_mint(mint)
token_acc.token_authority(authority)
token_acc.token_program(token_program)       # emits token::token_program

mint_acc.mint_decimals(9)
mint_acc.mint_authority(authority)
mint_acc.mint_freeze_authority(authority)    # emits mint::freeze_authority
mint_acc.mint_token_program(token_program)   # emits mint::token_program

ata.associated_token(mint=mint, authority=authority, token_program=token_program)
```

These map directly to Anchor’s documented SPL constraints and are allowed as checks with partial subsets. ([Docs.rs][2])

### Compiler implementation: where to hook

You implement these the same way `.has_one`/`.close`/`.realloc` are implemented today:

* In `src/core/compile/check/mod.rs`, in the account-type attribute handler (currently the `Ty::Anonymous(0)` branch), return a function type with a `Transformation` that returns `Transformed::AccountConstraint { ... }`.
* Give these transformations a context of `ExprContext::AccountAttr` when the arguments must be Anchor-scope.

Examples:

* `.constraint(expr)` should be `Transformation::new_with_context(..., Some(ExprContext::AccountAttr))`
* `.address(expr)` and `.owner(expr)` same
* `.seeds([...], bump=..., program=...)` needs **Seed context** for the seeds list itself (to reuse Seed casts) but AccountAttr context for bump/program/other expressions (you can either:

  * treat the whole call as Seed-context and explicitly rebuild bump/program under AccountAttr inside the transformation; or
  * split into two methods: `seeds([...])` and `seeds_program(...)`; I strongly prefer keeping one method and doing a mixed-context build inside the transform)

---

## 7) Step 4: Build-stage merging and inferred accounts

Right now `AccountInit` has special inference:

* always inserts `system_program`
* always inserts `rent` sysvar
* inserts `token_program` + `associated_token_program` depending on init type

You’ll generalize this to “scan constraints after merge”.

### 7.1 Inference rules you should implement (parity aligned)

* If any account has `Init` / `InitIfNeeded` → infer `system_program` (Anchor requires it). ([Docs.rs][2])
* If any account has `Realloc` → infer `system_program` (Anchor realloc uses lamports transfers). ([Docs.rs][2])
* If any account has `AssociatedToken(...)` → infer `associated_token_program` + `token_program` (and `system_program` if init)
* If any account uses `token::token_program` / `mint::token_program` / `associated_token::token_program` and the expression is a *field name*, you must ensure that field exists as an account (or infer the standard `token_program` if omitted and you want a default)

### 7.2 Fix current “duplicate inferred vs explicit” bug class

As written, Seahorse will happily insert `"system_program"` into `inferred_accounts` even if the user already declared `system_program` explicitly (leading to duplicate fields). Introduce:

```rust
impl InstructionContext {
  fn infer_account(&mut self, name: &str, account: ContextAccount) {
     let already_declared = self.accounts.iter().any(|(n, _)| n == name);
     if !already_declared {
         self.inferred_accounts.entry(name.to_string()).or_insert(account);
     }
  }
}
```

Then replace all direct `.inferred_accounts.insert(...)` calls with `infer_account`.

---

## 8) Step 5: Codegen (`#[account(...)]`) emission, including `@` custom errors

### 8.1 Emission ordering

Keep output stable for testing. Anchor doesn’t *require* strict ordering, but stable output matters for Seahorse snapshot tests.

A sensible canonical ordering:

1. `init` / `init_if_needed` / `zero` / `mut` / `signer` / `dup`
2. payer / space / rent_exempt
3. seeds + seeds::program + bump
4. SPL groups (mint/token/associated_token + token_program)
5. has_one/address/owner/executable/close/realloc/constraint

### 8.2 `@` custom errors

Anchor supports `@ <custom_error>` on several constraints (`mut`, `signer`, `dup`, etc.) in docs. ([Anchor Lang][1])

Even if Seahorse doesn’t *currently* generate Anchor error enums, you should architect constraints to allow it:

```rust
pub enum ConstraintError {
    Path(TypedExpression),    // e.g., MyError::Unauthorized
    Message(String),          // Seahorse-style auto-generated error (future)
}
```

And in codegen:

```rust
fn emit_flag(name: TokenStream, err: Option<&ConstraintError>) -> TokenStream {
   match err {
     None => quote!( #name ),
     Some(e) => quote!( #name @ #e ),
   }
}
```

Same for `key = value` constraints: `owner = expr @ err`.

This unlocks perfect parity later even if you initially only support `Path(...)`.

---

## 9) Step 6: Implement each missing core constraint (exact mapping)

Below is the “per-constraint implementation checklist” (what to implement, where, and what to generate).

### 9.1 `address = <expr>`

**Anchor:** `#[account(address = <pubkey_expr>)]` ([Anchor Lang][1])
**Seahorse:** `acct.address(expr, error=...)`

* **Typecheck hook:** method on base account type (`Ty::Anonymous(0)`).
* **Transform:** `Transformed::AccountConstraint { name, AccountConstraint::Address { expr, error } }`
* **Build merge:** attach constraint to `annotation`.
* **Codegen:** `address = #expr` (+ `@ err` if present)

### 9.2 `owner = <expr>`

**Anchor:** `#[account(owner = <expr>)]` ([Anchor Lang][1])
**Seahorse:** `acct.owner(expr, error=...)`

Same as address.

### 9.3 `executable`

**Anchor:** `#[account(executable)]` ([Anchor Lang][1])
**Seahorse:** `acct.executable()`

Flag constraint, no expr lowering needed.

### 9.4 `signer`

**Anchor:** `#[account(signer)]` (and `signer @ error`) ([Anchor Lang][1])
**Seahorse:** `acct.signer(error=...)`

This is important for parity when you want a signer that is *also* something else (e.g., `Account<MyData>` that must sign).

### 9.5 `dup`

**Anchor:** `#[account(dup)]` for duplicate **mutable** accounts ([Anchor Lang][1])
**Seahorse:** `acct.dup(error=...)`

**Extra validation you should add:**

* If account is read-only (`mut` not present), `dup` is pointless; reject or warn.
* If account is `init`/`zero` (implicitly mutable), allow.

### 9.6 `zero` + `rent_exempt`

**Anchor:** `#[account(zero)]` and can skip rent-exempt check via `rent_exempt = skip`. ([Docs.rs][2])
**Seahorse:** `acct.zero(rent_exempt="skip"|"enforce")`

Validation:

* `zero` incompatible with `init` and `init_if_needed`.
* `zero` implies mutable; don’t emit `mut` alongside unless Anchor allows it.

### 9.7 `rent_exempt = skip|enforce`

**Anchor:** documented as `rent_exempt = enforce|skip`. ([Docs.rs][2])
**Seahorse:** `acct.rent_exempt("skip")` or `acct.rent_exempt(skip=True)`

Implementation: store `RentExemptMode` in annotation and emit `rent_exempt = skip` or `rent_exempt = enforce`.

### 9.8 `constraint = <expr>`

**Anchor:** `#[account(constraint = <expr>)]` (+ `@ err`) ([Anchor Lang][1])
**Seahorse:** `acct.constraint(expr, error=...)`

This is the “expression lowering hard part”. Must build `expr` in `ExprContext::AccountAttr` so you don’t emit Seahorse runtime wrapper calls.

### 9.9 PDA `seeds`, `bump`, `bump = <expr>`, `seeds::program`

**Anchor:** supports `seeds = [...]`, `bump` and `bump = <expr>`, plus `seeds::program = <expr>`. ([Anchor Lang][1])
**Seahorse:**

* For init: extend `Empty.init(..., seeds=..., bump=..., seeds_program=...)`
* For non-init: `acct.seeds([...], bump=..., program=...)`

Validation:

* If `seeds::program` is set alongside `init`, reject (docs.rs says not allowed). ([Docs.rs][2])
* If `bump=expr` provided, do not assume `ctx.bumps.<name>` exists in wrapper code; either:

  * set `Empty.bump` from the instruction arg, or
  * only support explicit bump on non-Empty accounts initially.

### 9.10 `mut` parity

Anchor’s `mut` is optional and supports `mut @ error`. ([Anchor Lang][1])
Seahorse currently emits `mut` for basically everything non-init, which is not parity and causes real issues with duplicate signers.

You need a plan (next section).

---

## 10) The big parity blocker: `mut`/`dup` behavior in Seahorse

Anchor’s behavior:

* If accounts are **mutable**, duplicates are rejected unless `dup` is set. ([Anchor Lang][1])
* If accounts are **read-only**, duplicates are fine without `dup`.

Seahorse today:

* Treats almost all account-ish types as “mut” via `ty.is_mut() == true`.
* Therefore a *very normal Anchor pattern* (payer == authority) becomes invalid unless you add `dup`.

### Minimal viable parity fix (do this early)

Introduce explicit **read-only opt-out** on accounts that don’t need to be writable:

```py
authority.readonly()
payer.readonly()
```

and emit no `mut` for those.

This doesn’t require whole-program write analysis; it’s immediately useful.

### “Perfect” parity (the real fix)

Implement **writability inference + conditional store**:

1. **Track which accounts are written to**

   * Written-to if:

     * account is target of `.realloc`, `.close`, `.zero`, `.init`
     * account data fields are assigned (`acct.field = ...`)
     * account is used in a CPI helper that requires mut (token transfers, etc.) — you can conservatively mark as mut if it’s passed to known CPI wrappers
2. **Only emit `mut` if written-to**
3. **Only call `store()` on defined accounts that are written-to**

   * Right now Seahorse stores *all* defined accounts unconditionally in the wrapper, which effectively forces all defined accounts to be writable.

This is the key to:

* removing unnecessary `mut` flags
* reducing account metas
* avoiding `dup` boilerplate
* matching Anchor ergonomics

If you do only one “big” change for constraint parity: do this.

---

## 11) SPL constraints parity plan (token/mint/ata)

### 11.1 Support SPL constraints as checks, not just init

Anchor explicitly allows specifying partial subsets of token constraints as checks (e.g., only `token::mint` or only `token::authority`). ([Docs.rs][2])

Seahorse must allow setting these constraints on:

* already-initialized TokenAccount / TokenMint accounts
* associated token accounts

Implement methods:

```py
token_acc.token_mint(mint)
token_acc.token_authority(authority)
token_acc.token_program(token_program)

mint_acc.mint_decimals(9)
mint_acc.mint_authority(authority)
mint_acc.mint_freeze_authority(authority)
mint_acc.mint_token_program(token_program)

ata.associated_token(mint=mint, authority=authority, token_program=token_program)
```

### 11.2 Add token_program override constraints

Anchor includes group-specific `*::token_program` constraints. ([Docs.rs][2])
Emit:

* `token::token_program = ...`
* `mint::token_program = ...`
* `associated_token::token_program = ...`

### 11.3 Inference updates

If `associated_token(...)` constraint exists anywhere (init or not), infer `associated_token_program` and `token_program` accounts.

---

## 12) Token-2022 extensions constraints parity (phase plan)

Anchor documents a wide set of `extensions::...` constraints. ([Anchor Lang][1])

A realistic plan that doesn’t explode the Seahorse surface:

### Phase A (generic “extensions constraint bag”)

Add a compiler-internal representation:

```rust
pub struct ExtensionConstraints {
  pub items: Vec<(Vec<String>, TypedExpression)>;
  // e.g. (["extensions","transfer_hook","program_id"], expr)
}
```

Expose a low-level Seahorse API:

```py
mint.extension("transfer_hook.program_id", some_program.key())
mint.extension("close_authority.authority", authority.key())
```

Then emit tokens like:

* `extensions::transfer_hook::program_id = <expr>`
* `extensions::close_authority::authority = <expr>`

This gets you parity quickly without adding 20+ Python methods.

### Phase B (typed helpers)

Once stable, add typed wrappers for the most common ones.

### Phase C (TokenInterface accounts)

True Token-2022 parity usually also wants Anchor’s `InterfaceAccount` / TokenInterface patterns and the `*::token_program` overrides; that’s a broader type-system extension beyond constraints.

---

## 13) Update the Seahorse prelude stubs (DX parity)

Your `data/const/seahorse_prelude.py` currently does **not** declare methods that Seahorse already supports (`has_one`, `close`, `realloc`), so editors won’t autocomplete them.

As part of constraint parity, update it to include:

```py
class AccountWithKey:
    def key(self) -> Pubkey: ...
    def signer(self, error: Any = None): ...
    def dup(self, error: Any = None): ...
    def address(self, pk: Pubkey, error: Any = None): ...
    def owner(self, pk: Pubkey, error: Any = None): ...
    def executable(self): ...
    def zero(self, rent_exempt: str = "enforce"): ...
    def rent_exempt(self, mode: str): ...
    def seeds(self, seeds: List[Any], bump: Any = None, program: Any = None, error: Any = None): ...
    def constraint(self, expr: bool, error: Any = None): ...
    def close(self, recipient: 'AccountWithKey'): ...
    def has_one(self, target: 'AccountWithKey'): ...
    def realloc(self, size: u64, payer: 'Signer', zero: bool): ...
```

And similarly for TokenAccount / TokenMint constraint setters with names that don’t conflict with existing runtime methods.

---

## 14) Test plan (don’t skip this if you want “perfect”)

### Golden (snapshot) tests

For each Anchor constraint, create a tiny Seahorse program and snapshot the generated Rust:

* `address`, `owner`, `signer`, `dup`, `executable`, `zero`, `rent_exempt`, `constraint`
* `seeds` on non-init accounts, `bump = arg`, `seeds::program`
* token_program overrides
* SPL checks without init

### Compile tests against anchor derive

The most valuable tests are “generated Rust compiles under anchor” because attribute-token syntax is easy to get subtly wrong.

### Property tests for merge rules

Unit-test `AccountAnnotation::push/validate`:

* duplicates rejected
* mutual exclusions enforced
* seeds::program with init rejected ([Docs.rs][2])
* dup requires mut semantics consistent with Anchor docs ([Anchor Lang][1])

---

## 15) Quick “do this first” checklist (highest ROI)

If you want the shortest path to meaningful constraint parity:

1. **Add `ExprContext::AccountAttr`** and fix key()/borrow/move/literal behavior in that mode
2. Implement core missing constraints: `address`, `owner`, `signer`, `dup`, `constraint`, `zero`, `rent_exempt`, `seeds::program`, `bump = <expr>` ([Anchor Lang][1])
3. Implement seeds constraints on **non-init accounts** (PDA validation parity) ([Anchor Lang][1])
4. Implement SPL check constraints + token_program overrides ([Docs.rs][2])
5. Add `readonly()` (or equivalent) and **stop emitting `mut` everywhere**, otherwise `dup` becomes mandatory boilerplate for normal patterns ([Anchor Lang][1])

If you want, in the next round I can write this as a concrete PR-style patch plan (file-by-file) with the exact new enums/structs, method hooks in `check/mod.rs`, build merging changes, and codegen emission code (including a stable ordering function).

[1]: https://www.anchor-lang.com/docs/references/account-constraints "Account Constraints"
[2]: https://docs.rs/anchor-derive-accounts/latest/anchor_derive_accounts/derive.Accounts.html "Accounts in anchor_derive_accounts - Rust"
