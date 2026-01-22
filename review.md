Below is a combined **codebase audit + Anchor parity gap analysis** based on the zip you provided (Seahorse compiler + templates + examples + test harness + anchor-reference programs).

I’m going to be blunt and specific, with file-level pointers and concrete “here’s how I’d change it” diffs.

---

## 1) Quick snapshot: what Seahorse actually supports today (from the code)

### Pipeline (high level)

Seahorse is a Rust compiler that:

1. parses Python (via `rustpython_parser`),
2. “cleans” the AST into a restricted Seahorse AST,
3. preprocesses/import-resolves,
4. typechecks + records evaluation/assignment order,
5. “builds” a lower IR (statements + inferred accounts + account annotations),
6. generates Anchor Rust (one big `lib.rs`) and formats it.

You can see this structure reflected in:

* `src/core/clean/*`
* `src/core/preprocess/*`
* `src/core/compile/*` (typecheck + build + builtin modeling)
* `src/core/generate/mod.rs` (one very large codegen module)

### Built-in “Anchor-ish” features Seahorse has (confirmed in codegen)

From `AccountAnnotation` and the emitter in `src/core/generate/mod.rs`, Seahorse currently can generate these Anchor constraints:

* `mut` (but see “all accounts become mut” issue below)
* `init` / `init_if_needed`
* `payer`
* `space`
* `seeds = [...]` and `bump`
* token constraints:

  * `mint::decimals`
  * `mint::authority`
  * `token::mint`
  * `token::authority`
  * `associated_token::mint`
  * `associated_token::authority` *(attempted; see bug/limitation section)*
* `close = <recipient>`
* `realloc = <size>`, `realloc::payer = ...`, `realloc::zero = ...`
* `has_one = <target>`

These are emitted in `AccountAnnotationWithTyExpr::to_tokens()` in `src/core/generate/mod.rs`.

---

## 2) Anchor feature parity: what’s missing (big gaps)

Anchor’s constraint system is broader than what Seahorse emits today. The current Anchor docs list constraints like `signer`, `dup`, `address`, `owner`, `executable`, `zero`, `constraint = ...`, `seeds::program`, `bump = ...`, plus token-program overrides and Token-2022 extension constraints. ([Anchor Language][1])

### 2.1 Account constraints: parity matrix

Here’s the practical “what you can express” matrix:

```yaml
anchor_account_constraints:
  mut:              seahorse: "YES (but currently forced-on too often)"
  signer:           seahorse: "NO (only via using Signer type, not as a constraint on other accounts)"
  dup:              seahorse: "NO"
  init:             seahorse: "YES"
  init_if_needed:   seahorse: "YES"
  payer:            seahorse: "YES"
  space:            seahorse: "YES"
  seeds:            seahorse: "YES"
  bump:             seahorse: "YES (implicit bump only; no bump=<expr>)"
  seeds::program:   seahorse: "NO"
  bump=<expr>:      seahorse: "NO"
  has_one:          seahorse: "YES"
  close:            seahorse: "YES"
  realloc:          seahorse: "YES"
  realloc::payer:   seahorse: "YES"
  realloc::zero:    seahorse: "YES"
  address:          seahorse: "NO"
  owner:            seahorse: "NO"
  executable:       seahorse: "NO"
  zero:             seahorse: "NO"
  constraint=...:   seahorse: "NO"
```

This list is directly derived from what Anchor supports ([Anchor Language][1]) vs what Seahorse’s `AccountAnnotation` contains and what `AccountAnnotationWithTyExpr` emits.

#### Token-related constraints (Anchor supports far more)

Anchor’s docs include token-program override constraints (`token::token_program`, `mint::freeze_authority`, `associated_token::token_program`, etc.) and Token-2022 extension constraints. ([Anchor Language][1])
Seahorse does **not** emit those today (even when it includes the relevant program accounts).

---

### 2.2 Account types: Anchor supports types Seahorse can’t represent

Anchor’s account type system includes `Program<'info, T>`, `Interface<'info, T>`, `InterfaceAccount<'info, T>`, `AccountLoader<'info, T>` (zero-copy), `SystemAccount`, `Sysvar<'info, T>`, and optional accounts (`Option<...>`), among others. ([Anchor Language][2])

Seahorse today exposes only a subset in its prelude + codegen:

* `Account` (program-owned data)
* `Signer`
* `TokenMint`, `TokenAccount` (legacy SPL token)
* `Program` (but **codegen maps it to `UncheckedAccount<'info>`**, i.e. no ID validation)
* `UncheckedAccount`
* `Clock` sysvar (and `Rent` is inferred, not a first-class user type)

So parity gaps here:

* **No typed Program accounts**: Anchor `Program<'info, X>` validates program ID; Seahorse `Program` is effectively unchecked.
* **No interface accounts**: no `InterfaceAccount` / `Interface` (important for Token-2022 compatibility).
* **No AccountLoader / zero-copy accounts**.
* **No optional accounts**.

---

### 2.3 Context-level features: Seahorse doesn’t expose `remaining_accounts`, `program_id`, or full bump access

Anchor’s `Context` exposes:

* `program_id`
* `remaining_accounts`
* `bumps` (bump seeds found during validation) ([Anchor Language][3])

Seahorse doesn’t give the user a `ctx` at all; it passes only declared accounts + instruction args into the handler. So:

* **No `remaining_accounts` access** (major parity gap for many “dynamic account list” patterns).
* Bumps: Seahorse exposes bump only via `Empty[T].bump()` (and even that has a correctness issue; see below).

---

### 2.4 Errors: Seahorse does not have Anchor-style custom errors / `Result` flow

Anchor’s “native” model is:

* instruction handlers returning `Result<()>`
* custom errors via `#[error_code]`
* `require!` and friends, plus `err!` / `error!` macros ([Anchor Language][2])

Seahorse currently compiles:

* Python `assert` → **`panic!(...)`** (see `Statement::AnchorRequire` emitter)
* SPL CPIs → **`.unwrap()`** on results
* many other failure modes → panic/unwrap

That is a *very large* parity gap:

* Clients can’t reliably decode Anchor error codes
* You can’t attach error codes/messages in the Anchor way
* You can’t “return early with Err(...)” from Seahorse code

---

### 2.5 Tokens: no Token-2022 / token-interface parity

Anchor explicitly supports Token-2022 and token-interface patterns (`anchor_spl::token_interface`, `token_2022`, `token_2022_extensions`) ([Anchor Language][1])

Seahorse’s built-in token types map to legacy SPL Token only:

* `TokenMint` → `anchor_spl::token::Mint`
* `TokenAccount` → `anchor_spl::token::TokenAccount`
  …and there’s no built-in `InterfaceAccount` approach to handle either token program transparently.

Your own repo even notes: “Seahorse doesn’t have native Token-2022 support (only legacy SPL Token…)” (in `prd.json`).

---

## 3) Codebase audit: hacks / issues / shortcuts worth fixing

I’m going to group these into **correctness**, **security**, **parity blockers**, and **maintainability**.

---

## 3.1 Correctness issues in generated code / semantics

### (A) `assert` → `panic!` instead of `require!` + error codes

* Build stage converts Python `assert` statements into `Statement::AnchorRequire` (good intent).
* Codegen emits:

```rust
if !cond {
    panic!("msg");
}
```

This is in `src/core/generate/mod.rs` under `Statement::AnchorRequire`.

Why this matters:

* Anchor expects error returns, not panics, for normal failure.
* Panics can produce generic errors and prevent client-side error decoding.
* This blocks parity with `#[error_code]` and `require!` usage. ([Anchor Language][2])

**Fix direction** (requires a bigger shift; see roadmap):

* Make Seahorse handler functions return `Result<()>`
* Compile `assert` → `require!(cond, SomeError)` or `require_msg!` style equivalent
* Provide a Seahorse-side error definition mechanism that compiles to `#[error_code]`

---

### (B) CPI wrappers use `.unwrap()` everywhere

In `src/core/compile/builtin/prelude.rs`, token CPIs are emitted as:

* `token::transfer(...).unwrap();`
* `token::mint_to(...).unwrap();`
* etc.

This is okay for “prototype pythonic runtime semantics”, but it’s a **non-starter for Anchor parity** because real programs want to bubble errors via `?`.

**Fix direction**

* Once handlers return `Result<()>`, generate `token::transfer(...)?;`.
* For “pythonic feel”, you don’t need user-visible `Result`; just make the compiler generate it.

---

### (C) `Empty.bump` is *always* set to `Some(ctx.bumps.<name>)` (likely wrong)

In `make_lib` (codegen), `Empty` accounts are loaded with:

```rust
seahorse_util::Empty {
  account: &ctx.accounts.name,
  bump: Some(ctx.bumps.name),
}
```

This happens unconditionally for any `Empty[T]`.

Why this is likely incorrect:

* Anchor’s `ctx.bumps.<field>` exists for accounts where the derive macro tracks a bump (typically PDA constraints).
* If an `init` account has **no `seeds`**, there may be **no bump field**.
* For `associated_token` creation, bump behavior is also not guaranteed to be exposed via `ctx.bumps` (and your own `progress.txt` says `associated=True` generates invalid code).

This likely explains the “associated=True invalid code” note: your generated code can end up referencing a non-existent `ctx.bumps.<account>` field.

**Minimal patch idea (codegen side)**
Only set `bump: Some(...)` when the account actually has `seeds` (and maybe when it’s a known PDA-like constraint), otherwise `None`.

Sketch diff (in `src/core/generate/mod.rs`, inside `make_lib`’s `load_accounts` closure):

```diff
- let empty_obj = quote! { seahorse_util::Empty { account: &ctx.accounts.#name, bump: Some(ctx.bumps.#name) } };
+ let bump_expr = match annotation.as_ref().and_then(|a| a.seeds.as_ref()) {
+     Some(_) => quote! { Some(ctx.bumps.#name) },
+     None => quote! { None },
+ };
+ let empty_obj = quote! {
+   seahorse_util::Empty { account: &ctx.accounts.#name, bump: #bump_expr }
+ };
```

This aligns with your own Python prelude docstring: “If this account was created without seeds, calling bump will runtime error.”

---

### (D) Account sizing model is “padding-based” and will waste space / still easy to get wrong

Seahorse defaults `space` for program accounts to:

```rust
space = std::mem::size_of::<Ty>() + 8
```

…and then adds optional `padding`.

This is a pragmatic heuristic, but it is not feature-parity with Anchor’s `InitSpace` derive and `#[max_len]` sizing patterns.

**Parity gap**
Anchor has a standardized story for sizing variable-length fields via `InitSpace` and `max_len`. ([Anchor Language][1])
Seahorse has “padding” which is less precise and much easier to misuse.

**Fix direction**

* Either:

  1. implement a Seahorse-side `@init_space(max_len=...)` that generates `#[derive(InitSpace)]` + `#[max_len]` (best parity), or
  2. add a static check: if an account contains any dynamically-sized field and user did not specify `space`/`padding`, fail compilation with a targeted error.

---

## 3.2 Parity blockers inside the compiler

### (E) “All accounts are mut” (this is big)

In `src/core/compile/build/mod.rs`, when building instruction context params:

```rust
let mut account_annotation = AccountAnnotation::new();
account_annotation.is_mut = ty.is_mut();
```

But `ty.is_mut()` returns `true` for `Account`/`Signer`/`TokenAccount` etc (because Seahorse wants python-mutable semantics). Net result:

* **Nearly every declared account becomes `#[account(mut)]`**.

Consequences:

* Reduced parallelism (everything is write-locked).
* Many valid Anchor patterns become invalid because “duplicate readonly accounts” become “duplicate mutable accounts”.
* Anchor has a `dup` constraint for intentional duplicate mut accounts. Seahorse doesn’t support `dup`. ([Anchor Language][1])
  So you’re stuck in the worst of both worlds.

**Fix direction options**

1. **Add a user escape hatch** (fastest):

   * introduce `Readonly[T]` / `Const[T]` wrapper, similar to `Empty[T]`.
   * or allow `@readonly(account_name)` decorator on instruction.
2. **Do real mutability inference** (best):

   * track writes to account fields / `.store()` / `.realloc()` / `.close()` / token operations that require writable accounts
   * only mark those as mut.

Even a conservative “mark mut only if written anywhere” will improve.

---

### (F) `Program` type is generated as `UncheckedAccount<'info>`

In `make_ty_expr` (build → TyExpr), `Prelude::Program` and `Prelude::UncheckedAccount` both map to the same stored type:

```rust
UncheckedAccount<'info>
```

This means Seahorse cannot express Anchor’s `Program<'info, SomeProgram>` validation. That’s a safety gap and parity gap. ([Anchor Language][2])

**Fix direction**
Introduce a generic `Program[T]` type in Seahorse:

Python-ish:

```py
from seahorse.prelude import Program

other: Program[SomeOtherProgram]
```

Codegen should map to:

```rust
pub other: Program<'info, SomeOtherProgram>
```

This requires:

* new prelude builtin
* TyExpression generic support already exists (Empty[T]); reuse that machinery.

---

### (G) Missing constraint system features in `AccountAnnotation`

Your `AccountAnnotation` (in `src/core/compile/ast.rs`) is missing fields for:

* `address`
* `owner`
* `executable`
* `zero`
* `constraint` expressions
* `dup`
* `seeds::program`
* explicit bump capture (`bump = ...`)
* token program override constraints (`token::token_program`, `associated_token::token_program`)
* mint freeze authority (`mint::freeze_authority`)
* Token-2022 extension constraints

All of these exist in Anchor’s constraint system. ([Anchor Language][1])

---

## 3.3 “Hacks” / shortcuts in the Rust compiler itself

### (H) `Transformation` equality is hardcoded to “always equal”

In `src/core/compile/build/mod.rs`:

```rust
impl PartialEq for Transformation {
    fn eq(&self, _: &Self) -> bool { true }
    fn ne(&self, _: &Self) -> bool { false }
}
```

This is a classic “I need PartialEq for a struct that contains closures” hack.

Risk:

* If any code starts depending on transformations for caching/memoization/equality, you’ll get silent incorrectness.
* It also makes debugging typecheck outputs harder (Debug prints “Transformation”).

**Fix direction**
Prefer one of:

* Remove `PartialEq` requirements for structures containing transformations.
* Make `Transformation` carry a stable ID/tag (e.g., enum variant + parameters) and compare on that.
* Implement `PartialEq` for `Ty` such that `Transformed(_, _)` compares only on the inner type *and* never compares the transformation itself (explicitly), rather than poisoning `Transformation` globally.

---

### (I) Panics/unwraps in compiler code paths that can be hit by user input

There are many `unwrap()` and `panic!()` in compiler stages (not just codegen). Some are internal invariants; some will be triggered by malformed user code.

Example patterns:

* `match1!` macro panics if the AST shape isn’t what you expected (`src/core/util.rs`)
* lots of `.unwrap()` after lookups

**Fix direction**

* Convert panics into `CoreError` wherever user input could trigger it.
* Keep panics only where you’re truly in “compiler bug” territory, but even then consider `catch_unwind` at CLI boundary to print a friendly “compiler bug” report with context.

---

## 3.4 CLI security & robustness issues

### (J) Path traversal / dangerous deletion in `seahorse build`

In `src/bin/cli/build.rs`, the build step does:

```rust
let src = project_path.join("programs").join(program_name).join("src");
remove_dir_all(&src)?;
```

If `program_name` is user-provided (it is), and contains `..` segments, you can potentially delete outside the intended directory tree.

**Fix direction (minimum viable)**
Reject program names containing path separators or `..`.

```diff
+ if program_name.contains('/') || program_name.contains('\\') || program_name.contains("..") {
+     return Err(anyhow!("invalid program name"));
+ }
```

Better: enforce Anchor crate name regex.

---

## 4) Anchor features not supported in Seahorse (condensed list)

Here’s the “feature parity checklist” you asked for, focusing on major Anchor features that are either missing or meaningfully incomplete:

### 4.1 Constraints missing

From Anchor constraints docs ([Anchor Language][1]):

* `signer` *(as a constraint, not just Signer type)*
* `dup`
* `address`
* `owner`
* `executable`
* `zero`
* `constraint = <expr>` and `@ <error>` mapping
* `seeds::program`
* `bump = <expr>` (explicit bump capture)
* token program override constraints:

  * `token::token_program`
  * `associated_token::token_program`
* mint freeze authority:

  * `mint::freeze_authority`
* Token-2022 extension constraints (various `extensions::...`)

### 4.2 Account types missing

From Anchor account types docs ([Anchor Language][2]):

* `Program<'info, T>` (typed + ID checked)
* `Interface<'info, T>` / `InterfaceAccount<'info, T>`
* `AccountLoader<'info, T>` / “zero-copy” style accounts
* `SystemAccount<'info>`
* generic `Sysvar<'info, T>` access patterns beyond Clock (and others)
* optional accounts: `Option<Account<...>>`, etc.

### 4.3 Context capabilities missing

From Anchor’s `Context` struct docs ([Anchor Language][3]):

* `remaining_accounts`
* direct `program_id` access
* `bumps` access generally (beyond the `Empty.bump()` pattern)

### 4.4 Errors and error codes missing

From Anchor custom error docs ([Anchor Language][2]):

* `#[error_code]` + `#[msg(...)]`
* `err!`, `error!`
* `require!` family (as actual error returns, not panics)

### 4.5 Token-2022 / interface-based token support missing

Anchor token docs and the “Create Token Account” guide explicitly cover token-interface + Token-2022 patterns ([Anchor Language][1]). Seahorse doesn’t have:

* `token_interface`-based account types
* Token-2022 mint/account types
* Token-2022 extension support

---

## 5) Concrete “make it perfect” roadmap (high ROI order)

If the goal is true parity with modern Anchor, here’s the order I’d attack it:

### Phase 1 — Stop panicking, start returning `Result<()>` (unblocks tons of parity)

1. Change Seahorse-generated handlers to return `Result<()>`
2. Replace `.unwrap()` in generated CPI calls with `?`
3. Replace `AnchorRequire` emission from `panic!` to `require!` + a real error system ([Anchor Language][2])
4. Add a Seahorse syntax for defining errors that compiles to `#[error_code]`

Possible Seahorse syntax idea:

```py
@error_code
class Errors(Enum):
    Unauthorized = 6000
    InvalidInput = 6001

# usage
require(user.key() == admin.key(), Errors.Unauthorized)
```

### Phase 2 — Expand `AccountAnnotation` to cover core constraints

Add fields + syntax + emit support for:

* `address`, `owner`, `executable`, `zero`, `dup`, `constraint`
* `seeds::program`
* token-program override constraints (`token::token_program`, `associated_token::token_program`) ([Anchor Language][1])
* mint freeze authority

You already have a pattern for “declarative constraints via method calls” (`.close()`, `.realloc()`, `.has_one()`). Extend it:

```py
some_account.owner(system_program.key())    # -> owner=...
some_account.address(expected_pubkey)       # -> address=...
some_account.executable()                   # -> executable
some_account.dup()                          # -> dup
some_account.constraint(some_account.x > 0) # -> constraint=...
```

### Phase 3 — Fix mutability + duplicates

* Introduce `Readonly[T]` or do write-inference.
* Implement Anchor `dup` constraint (otherwise you’ll always be blocked by duplicates once you stop over-marking mut). ([Anchor Language][1])

### Phase 4 — Token interface + Token-2022

* Add prelude types that map to `InterfaceAccount<'info, token_interface::Mint/TokenAccount>` and `Interface<'info, TokenInterface>` ([Anchor Language][4])
* Add constraints emission for `token::token_program` and `associated_token::token_program` to support both Token Program and Token Extension Program. ([Anchor Language][4])
* Add Token-2022 extensions support if you want parity with current `anchor_spl`. ([Anchor Language][1])

### Phase 5 — Context features

* Provide Seahorse access to `remaining_accounts` (maybe as an implicit arg `remaining_accounts: List[UncheckedAccount]`)
* Provide `program_id` and full `bumps` map access (not just `Empty.bump()`) ([Anchor Language][3])

### Phase 6 — Zero-copy & AccountLoader (optional but parity with advanced Anchor)

* Add `@zero_copy` decorator or account-level annotation and generate `AccountLoader<'info, T>` ([Anchor Language][2])
* Provide fixed-size array types and forbid dynamic fields, or support max_len annotations.

---

## 6) Three “easy wins” I’d implement immediately

### Win #1: Fix `Empty.bump` correctness (conditional bump)

This is both a correctness fix and unlocks associated-token work.

### Win #2: Add `UncheckedAccount -> Seed` cast

This removes an annoying limitation you already documented (“UncheckedAccount in signer seeds”).

Where to implement:

* `src/core/compile/builtin/prelude.rs` in `fn casted()`, add UncheckedAccount to Seed transform.

### Win #3: Sanitize `program_name` in CLI build

This is a genuine safety issue.

---

## 7) If you want, I can go deeper in two directions

1. **Constraint parity implementation plan**: I can outline exactly what to change in:

   * `AccountAnnotation` (fields),
   * `check` layer (method typing + transformations),
   * `build` layer (recording annotations),
   * `generate` layer (emission),
     with suggested Seahorse surface syntax for each Anchor constraint.

2. **Error system design**: propose a Seahorse-native error declaration & raising model that compiles cleanly to Anchor `#[error_code]` + `require!`/`err!`, without making Seahorse users write Rust-ish `Result`.

If you tell me which you want first (constraints vs errors), I’ll produce a concrete design + code-level patch plan.

[1]: https://www.anchor-lang.com/docs/references/account-constraints "Account Constraints"
[2]: https://www.anchor-lang.com/docs/features/errors "Custom Errors"
[3]: https://www.anchor-lang.com/docs/tokens "Token Integration with Anchor"
[4]: https://www.anchor-lang.com/docs/tokens/basics/create-token-account "Create a Token Account"
