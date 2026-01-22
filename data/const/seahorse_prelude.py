# seahorse.prelude: the basis for writing Seahorse programs.
#
# NOTE: this file just contains types and documentation for your editor. This
# is NOT executable code, and you won't be able to change the behavior of your
# Seahorse programs by editing this file.

from typing import *
from math import floor, ceil

T = TypeVar('T')
N = TypeVar('N')


# ==========
# Rust types
# ==========

class u8:
    """8-bit unsigned integer."""

    def __init__(self, _: Any) -> 'u8':
        """Construct an u8."""

    def __add__(self, other: 'u8') -> 'u8':
        pass

    def __radd__(self, other: 'u8') -> 'u8':
        pass

    def __iadd__(self, other: 'u8') -> 'u8':
        pass

    def __sub__(self, other: 'u8') -> 'u8':
        pass

    def __rsub__(self, other: 'u8') -> 'u8':
        pass

    def __isub__(self, other: 'u8') -> 'u8':
        pass

    def __mul__(self, other: 'u8') -> 'u8':
        pass

    def __rmul__(self, other: 'u8') -> 'u8':
        pass

    def __imul__(self, other: 'u8') -> 'u8':
        pass

    def __truediv__(self, other: 'f64') -> 'f64':
        pass

    def __rtruediv__(self, other: 'f64') -> 'f64':
        pass

    def __itruediv__(self, other: 'f64') -> 'f64':
        pass

    def __floordiv__(self, other: 'u8') -> 'u8':
        pass

    def __rfloordiv__(self, other: 'u8') -> 'u8':
        pass

    def __ifloordiv__(self, other: 'u8') -> 'u8':
        pass

    def __lt__(self, other: 'u8') -> bool:
        pass

    def __le__(self, other: 'u8') -> bool:
        pass

    def __eq__(self, other: 'u8') -> bool:
        pass

    def __ne__(self, other: 'u8') -> bool:
        pass

    def __ge__(self, other: 'u8') -> bool:
        pass

    def __gt__(self, other: 'u8') -> bool:
        pass


class u16:
    """16-bit unsigned integer."""

    def __init__(self, _: Any) -> 'u16':
        """Construct an u16."""

    def __add__(self, other: 'u16') -> 'u16':
        pass

    def __radd__(self, other: 'u16') -> 'u16':
        pass

    def __iadd__(self, other: 'u16') -> 'u16':
        pass

    def __sub__(self, other: 'u16') -> 'u16':
        pass

    def __rsub__(self, other: 'u16') -> 'u16':
        pass

    def __isub__(self, other: 'u16') -> 'u16':
        pass

    def __mul__(self, other: 'u16') -> 'u16':
        pass

    def __rmul__(self, other: 'u16') -> 'u16':
        pass

    def __imul__(self, other: 'u16') -> 'u16':
        pass

    def __truediv__(self, other: 'f64') -> 'f64':
        pass

    def __rtruediv__(self, other: 'f64') -> 'f64':
        pass

    def __itruediv__(self, other: 'f64') -> 'f64':
        pass

    def __floordiv__(self, other: 'u16') -> 'u16':
        pass

    def __rfloordiv__(self, other: 'u16') -> 'u16':
        pass

    def __ifloordiv__(self, other: 'u16') -> 'u16':
        pass

    def __lt__(self, other: 'u16') -> bool:
        pass

    def __le__(self, other: 'u16') -> bool:
        pass

    def __eq__(self, other: 'u16') -> bool:
        pass

    def __ne__(self, other: 'u16') -> bool:
        pass

    def __ge__(self, other: 'u16') -> bool:
        pass

    def __gt__(self, other: 'u16') -> bool:
        pass

class u32:
    """32-bit unsigned integer."""

    def __init__(self, _: Any) -> 'u32':
        """Construct an u32."""

    def __add__(self, other: 'u32') -> 'u32':
        pass

    def __radd__(self, other: 'u32') -> 'u32':
        pass

    def __iadd__(self, other: 'u32') -> 'u32':
        pass

    def __sub__(self, other: 'u32') -> 'u32':
        pass

    def __rsub__(self, other: 'u32') -> 'u32':
        pass

    def __isub__(self, other: 'u32') -> 'u32':
        pass

    def __mul__(self, other: 'u32') -> 'u32':
        pass

    def __rmul__(self, other: 'u32') -> 'u32':
        pass

    def __imul__(self, other: 'u32') -> 'u32':
        pass

    def __truediv__(self, other: 'f64') -> 'f64':
        pass

    def __rtruediv__(self, other: 'f64') -> 'f64':
        pass

    def __itruediv__(self, other: 'f64') -> 'f64':
        pass

    def __floordiv__(self, other: 'u32') -> 'u32':
        pass

    def __rfloordiv__(self, other: 'u32') -> 'u32':
        pass

    def __ifloordiv__(self, other: 'u32') -> 'u32':
        pass

    def __lt__(self, other: 'u32') -> bool:
        pass

    def __le__(self, other: 'u32') -> bool:
        pass

    def __eq__(self, other: 'u32') -> bool:
        pass

    def __ne__(self, other: 'u32') -> bool:
        pass

    def __ge__(self, other: 'u32') -> bool:
        pass

    def __gt__(self, other: 'u32') -> bool:
        pass

class u64:
    """64-bit unsigned integer."""

    def __init__(self, _: Any) -> 'u64':
        """Construct an u64."""

    def __add__(self, other: 'u64') -> 'u64':
        pass

    def __radd__(self, other: 'u64') -> 'u64':
        pass

    def __iadd__(self, other: 'u64') -> 'u64':
        pass

    def __sub__(self, other: 'u64') -> 'u64':
        pass

    def __rsub__(self, other: 'u64') -> 'u64':
        pass

    def __isub__(self, other: 'u64') -> 'u64':
        pass

    def __mul__(self, other: 'u64') -> 'u64':
        pass

    def __rmul__(self, other: 'u64') -> 'u64':
        pass

    def __imul__(self, other: 'u64') -> 'u64':
        pass

    def __truediv__(self, other: 'f64') -> 'f64':
        pass

    def __rtruediv__(self, other: 'f64') -> 'f64':
        pass

    def __itruediv__(self, other: 'f64') -> 'f64':
        pass

    def __floordiv__(self, other: 'u64') -> 'u64':
        pass

    def __rfloordiv__(self, other: 'u64') -> 'u64':
        pass

    def __ifloordiv__(self, other: 'u64') -> 'u64':
        pass

    def __lt__(self, other: 'u64') -> bool:
        pass

    def __le__(self, other: 'u64') -> bool:
        pass

    def __eq__(self, other: 'u64') -> bool:
        pass

    def __ne__(self, other: 'u64') -> bool:
        pass

    def __ge__(self, other: 'u64') -> bool:
        pass

    def __gt__(self, other: 'u64') -> bool:
        pass

class u128:
    """128-bit unsigned integer."""

    def __init__(self, _: Any) -> 'u128':
        """Construct an u128."""

    def __add__(self, other: 'u128') -> 'u128':
        pass

    def __radd__(self, other: 'u128') -> 'u128':
        pass

    def __iadd__(self, other: 'u128') -> 'u128':
        pass

    def __sub__(self, other: 'u128') -> 'u128':
        pass

    def __rsub__(self, other: 'u128') -> 'u128':
        pass

    def __isub__(self, other: 'u128') -> 'u128':
        pass

    def __mul__(self, other: 'u128') -> 'u128':
        pass

    def __rmul__(self, other: 'u128') -> 'u128':
        pass

    def __imul__(self, other: 'u128') -> 'u128':
        pass

    def __truediv__(self, other: 'f64') -> 'f64':
        pass

    def __rtruediv__(self, other: 'f64') -> 'f64':
        pass

    def __itruediv__(self, other: 'f64') -> 'f64':
        pass

    def __floordiv__(self, other: 'u128') -> 'u128':
        pass

    def __rfloordiv__(self, other: 'u128') -> 'u128':
        pass

    def __ifloordiv__(self, other: 'u128') -> 'u128':
        pass

    def __lt__(self, other: 'u128') -> bool:
        pass

    def __le__(self, other: 'u128') -> bool:
        pass

    def __eq__(self, other: 'u128') -> bool:
        pass

    def __ne__(self, other: 'u128') -> bool:
        pass

    def __ge__(self, other: 'u128') -> bool:
        pass

    def __gt__(self, other: 'u128') -> bool:
        pass

class i8:
    """8-bit signed integer."""

    def __init__(self, _: Any) -> 'i8':
        """Construct an i8."""

    def __add__(self, other: 'i8') -> 'i8':
        pass

    def __radd__(self, other: 'i8') -> 'i8':
        pass

    def __iadd__(self, other: 'i8') -> 'i8':
        pass

    def __sub__(self, other: 'i8') -> 'i8':
        pass

    def __rsub__(self, other: 'i8') -> 'i8':
        pass

    def __isub__(self, other: 'i8') -> 'i8':
        pass

    def __mul__(self, other: 'i8') -> 'i8':
        pass

    def __rmul__(self, other: 'i8') -> 'i8':
        pass

    def __imul__(self, other: 'i8') -> 'i8':
        pass

    def __truediv__(self, other: 'f64') -> 'f64':
        pass

    def __rtruediv__(self, other: 'f64') -> 'f64':
        pass

    def __itruediv__(self, other: 'f64') -> 'f64':
        pass

    def __floordiv__(self, other: 'i8') -> 'i8':
        pass

    def __rfloordiv__(self, other: 'i8') -> 'i8':
        pass

    def __ifloordiv__(self, other: 'i8') -> 'i8':
        pass

class i16:
    """16-bit signed integer."""

    def __init__(self, _: Any) -> 'i16':
        """Construct an i16."""

    def __add__(self, other: 'i16') -> 'i16':
        pass

    def __radd__(self, other: 'i16') -> 'i16':
        pass

    def __iadd__(self, other: 'i16') -> 'i16':
        pass

    def __sub__(self, other: 'i16') -> 'i16':
        pass

    def __rsub__(self, other: 'i16') -> 'i16':
        pass

    def __isub__(self, other: 'i16') -> 'i16':
        pass

    def __mul__(self, other: 'i16') -> 'i16':
        pass

    def __rmul__(self, other: 'i16') -> 'i16':
        pass

    def __imul__(self, other: 'i16') -> 'i16':
        pass

    def __truediv__(self, other: 'f64') -> 'f64':
        pass

    def __rtruediv__(self, other: 'f64') -> 'f64':
        pass

    def __itruediv__(self, other: 'f64') -> 'f64':
        pass

    def __floordiv__(self, other: 'i16') -> 'i16':
        pass

    def __rfloordiv__(self, other: 'i16') -> 'i16':
        pass

    def __ifloordiv__(self, other: 'i16') -> 'i16':
        pass

    def __lt__(self, other: 'i16') -> bool:
        pass

    def __le__(self, other: 'i16') -> bool:
        pass

    def __eq__(self, other: 'i16') -> bool:
        pass

    def __ne__(self, other: 'i16') -> bool:
        pass

    def __ge__(self, other: 'i16') -> bool:
        pass

    def __gt__(self, other: 'i16') -> bool:
        pass

class i32:
    """32-bit signed integer."""

    def __init__(self, _: Any) -> 'i32':
        """Construct an i32."""

    def __add__(self, other: 'i32') -> 'i32':
        pass

    def __radd__(self, other: 'i32') -> 'i32':
        pass

    def __iadd__(self, other: 'i32') -> 'i32':
        pass

    def __sub__(self, other: 'i32') -> 'i32':
        pass

    def __rsub__(self, other: 'i32') -> 'i32':
        pass

    def __isub__(self, other: 'i32') -> 'i32':
        pass

    def __mul__(self, other: 'i32') -> 'i32':
        pass

    def __rmul__(self, other: 'i32') -> 'i32':
        pass

    def __imul__(self, other: 'i32') -> 'i32':
        pass

    def __truediv__(self, other: 'f64') -> 'f64':
        pass

    def __rtruediv__(self, other: 'f64') -> 'f64':
        pass

    def __itruediv__(self, other: 'f64') -> 'f64':
        pass

    def __floordiv__(self, other: 'i32') -> 'i32':
        pass

    def __rfloordiv__(self, other: 'i32') -> 'i32':
        pass

    def __ifloordiv__(self, other: 'i32') -> 'i32':
        pass

    def __lt__(self, other: 'i32') -> bool:
        pass

    def __le__(self, other: 'i32') -> bool:
        pass

    def __eq__(self, other: 'i32') -> bool:
        pass

    def __ne__(self, other: 'i32') -> bool:
        pass

    def __ge__(self, other: 'i32') -> bool:
        pass

    def __gt__(self, other: 'i32') -> bool:
        pass

class i64:
    """64-bit signed integer."""

    def __init__(self, _: Any) -> 'i64':
        """Construct an i64."""

    def __add__(self, other: 'i64') -> 'i64':
        pass

    def __radd__(self, other: 'i64') -> 'i64':
        pass

    def __iadd__(self, other: 'i64') -> 'i64':
        pass

    def __sub__(self, other: 'i64') -> 'i64':
        pass

    def __rsub__(self, other: 'i64') -> 'i64':
        pass

    def __isub__(self, other: 'i64') -> 'i64':
        pass

    def __mul__(self, other: 'i64') -> 'i64':
        pass

    def __rmul__(self, other: 'i64') -> 'i64':
        pass

    def __imul__(self, other: 'i64') -> 'i64':
        pass

    def __truediv__(self, other: 'f64') -> 'f64':
        pass

    def __rtruediv__(self, other: 'f64') -> 'f64':
        pass

    def __itruediv__(self, other: 'f64') -> 'f64':
        pass

    def __floordiv__(self, other: 'i64') -> 'i64':
        pass

    def __rfloordiv__(self, other: 'i64') -> 'i64':
        pass

    def __ifloordiv__(self, other: 'i64') -> 'i64':
        pass

    def __lt__(self, other: 'i64') -> bool:
        pass

    def __le__(self, other: 'i64') -> bool:
        pass

    def __eq__(self, other: 'i64') -> bool:
        pass

    def __ne__(self, other: 'i64') -> bool:
        pass

    def __ge__(self, other: 'i64') -> bool:
        pass

    def __gt__(self, other: 'i64') -> bool:
        pass

class i128:
    """128-bit signed integer."""

    def __init__(self, _: Any) -> 'i128':
        """Construct an i128."""

    def __add__(self, other: 'i128') -> 'i128':
        pass

    def __radd__(self, other: 'i128') -> 'i128':
        pass

    def __iadd__(self, other: 'i128') -> 'i128':
        pass

    def __sub__(self, other: 'i128') -> 'i128':
        pass

    def __rsub__(self, other: 'i128') -> 'i128':
        pass

    def __isub__(self, other: 'i128') -> 'i128':
        pass

    def __mul__(self, other: 'i128') -> 'i128':
        pass

    def __rmul__(self, other: 'i128') -> 'i128':
        pass

    def __imul__(self, other: 'i128') -> 'i128':
        pass

    def __truediv__(self, other: 'f64') -> 'f64':
        pass

    def __rtruediv__(self, other: 'f64') -> 'f64':
        pass

    def __itruediv__(self, other: 'f64') -> 'f64':
        pass

    def __floordiv__(self, other: 'i128') -> 'i128':
        pass

    def __rfloordiv__(self, other: 'i128') -> 'i128':
        pass

    def __ifloordiv__(self, other: 'i128') -> 'i128':
        pass

    def __lt__(self, other: 'i128') -> bool:
        pass

    def __le__(self, other: 'i128') -> bool:
        pass

    def __eq__(self, other: 'i128') -> bool:
        pass

    def __ne__(self, other: 'i128') -> bool:
        pass

    def __ge__(self, other: 'i128') -> bool:
        pass

    def __gt__(self, other: 'i128') -> bool:
        pass

class f64:
    """64-bit floating point number."""

    def __init__(self, _: Any) -> 'f64':
        """Construct an f64."""

    def __add__(self, other: 'f64') -> 'f64':
        pass

    def __radd__(self, other: 'f64') -> 'f64':
        pass

    def __iadd__(self, other: 'f64') -> 'f64':
        pass

    def __sub__(self, other: 'f64') -> 'f64':
        pass

    def __rsub__(self, other: 'f64') -> 'f64':
        pass

    def __isub__(self, other: 'f64') -> 'f64':
        pass

    def __mul__(self, other: 'f64') -> 'f64':
        pass

    def __rmul__(self, other: 'f64') -> 'f64':
        pass

    def __imul__(self, other: 'f64') -> 'f64':
        pass

    def __truediv__(self, other: 'f64') -> 'f64':
        pass

    def __rtruediv__(self, other: 'f64') -> 'f64':
        pass

    def __itruediv__(self, other: 'f64') -> 'f64':
        pass

    def __floordiv__(self, other: 'f64') -> 'f64':
        pass

    def __rfloordiv__(self, other: 'f64') -> 'f64':
        pass

    def __ifloordiv__(self, other: 'f64') -> 'f64':
        pass

    def __lt__(self, other: 'f64') -> bool:
        pass

    def __le__(self, other: 'f64') -> bool:
        pass

    def __eq__(self, other: 'f64') -> bool:
        pass

    def __ne__(self, other: 'f64') -> bool:
        pass

    def __ge__(self, other: 'f64') -> bool:
        pass

    def __gt__(self, other: 'f64') -> bool:
        pass

class Array(Generic[T, N]):
    """
    A fixed-length array: contains type T and has size N.

    N must be known at compile-time, and may not be anything other than a non-negative integer literal. Example:

    ```
    # Good
    a: Array[u8, 4]

    # Bad
    N = 4
    a: Array[u8, N]
    ```
    """

    def __init__(iterable: Iterable[T], len: N) -> 'Array[T, N]':
        """
        Construct an array from an iterable and a length.

        The parameter len must be known at compile-time, and may not be anything other than a non-negative integer literal. Example:

        ```
        a = [0, 1, 2, 3]

        # Good
        Array(a, 4)
        # Compiles, but will definitely error at runtime
        Array(a, 5)

        # Bad (will not compile)
        a = [0, 1, 2, 3]
        Array(a, len(a))
        ```
        """

    def __getitem__(self, index: Any) -> T:
        """
        Index into this array.
        
        Like Python's native list type, performs wrapping indexing - if you pass in -1, you'll get the last element of the array.
        """

def array(*elements: T) -> Array[T, N]:
    """
    Create an array from a variadic list of elements. Example:

    ```
    # Array[u64, 3]
    array(u64(0), 1, 2)

    # Array[str, 4]
    array('seahorse', 'is', 'the', 'best!')
    ```
    """

class Enum:
    """
    A type that can have one of multiple named values.

    Note that unlike Rust enums, these cannot contain any data (other than the variant itself). Example:

    ```
    class MyEnum(Enum):
        ONE = 1
        TWO = 2
        THREE = 3

    @instruction
    def use_enum(code: MyEnum):
        if code == MyEnum.ONE:
            print(1)
        # ...
    ```
    """


# ============
# Solana types
# ============

class Pubkey:
    """32-byte account identifier."""

    def find_program_address(seeds: List[Any], program_id: 'Pubkey' = None) -> Tuple['Pubkey', u8]:
        """
        Find a valid program derived address and its corresponding bump seed. Calls the same function from Solana's Pubkey struct - read more [here](https://docs.rs/solana-program/latest/solana_program/pubkey/struct.Pubkey.html#method.find_program_address).
        
        @param seeds: A list of parameters to uniquely identify this account among all accounts created by your program. These may be string literals, other accounts, integers, or lists of bytes.
        @param program_id: The pubkey of the program that the PDA belongs to. Defaults to the current program's key.
        @returns: The canonical pubkey and bump seed.
        """

class AccountWithKey:
    """Generic Solana account."""

    def key(self) -> Pubkey:
        """Get this account's key."""

    def readonly(self) -> 'AccountWithKey':
        """
        Mark this account as readonly (no mut constraint).

        By default, Seahorse marks all accounts as mutable. Use this method
        to explicitly mark an account as readonly, which removes the `mut`
        constraint in the generated Anchor code.

        This is important for:
        - Allowing duplicate accounts (readonly duplicates don't need `dup`)
        - Signers that don't need to be writable (e.g., authority accounts)
        - Config/state accounts that are only read, not modified

        Example:
            @instruction
            def process(
                authority: Signer,
                config: Config,
            ):
                authority.readonly()  # Signer doesn't need to be writable
                config.readonly()     # Just reading config

        This generates:
            #[derive(Accounts)]
            pub struct Process<'info> {
                pub authority: Signer<'info>,  // No mut!
                pub config: Account<'info, Config>,  // No mut!
            }

        @returns: Self for method chaining.
        """

    def executable(self) -> 'AccountWithKey':
        """
        Mark this account as executable (i.e., it must be a program).

        This adds the Anchor #[account(executable)] constraint which verifies
        at runtime that the account is an executable program.

        @returns: Self for method chaining.
        """

    def address(self, pubkey: Pubkey) -> 'AccountWithKey':
        """
        Require this account to have a specific address (pubkey).

        This adds the Anchor #[account(address = <pubkey>)] constraint which verifies
        at runtime that the account's key matches the expected pubkey.

        @param pubkey: The expected pubkey for this account.
        @returns: Self for method chaining.
        """

    def owner(self, pubkey: Pubkey) -> 'AccountWithKey':
        """
        Require this account to be owned by a specific program.

        This adds the Anchor #[account(owner = <pubkey>)] constraint which verifies
        at runtime that the account's owner matches the expected program pubkey.

        @param pubkey: The expected owner (program) pubkey for this account.
        @returns: Self for method chaining.
        """

    def signer(self) -> 'AccountWithKey':
        """
        Mark this account as requiring signer authority.

        This adds the Anchor #[account(signer)] constraint which verifies
        at runtime that the account has signed the transaction.

        This is useful when you have an Account<'info, MyData> that also
        needs to sign the transaction.

        @returns: Self for method chaining.
        """

    def zero(self) -> 'AccountWithKey':
        """
        Mark this account as requiring zero-initialization (pre-allocated and zeroed).

        This adds the Anchor #[account(zero)] constraint which is used for accounts
        that were pre-allocated (zeroed) in a previous transaction and now need to
        be initialized. This is an alternative to using `init` when you want to
        separate the allocation and initialization steps.

        Note: The `zero` constraint is mutually exclusive with `init` and `init_if_needed`.

        @returns: Self for method chaining.
        """

    def dup(self) -> 'AccountWithKey':
        """
        Allow this account to be a duplicate of another mutable account in the instruction.

        By default, Anchor rejects instructions where the same mutable account is passed
        multiple times. This constraint explicitly allows duplicate mutable accounts.

        This adds the Anchor #[account(mut, dup)] constraint.

        @returns: Self for method chaining.
        """

    def constraint(self, expr: bool) -> 'AccountWithKey':
        """
        Add an arbitrary constraint expression to this account.

        The expression is evaluated during account validation and must return True
        for the instruction to proceed. This is the most flexible validation method,
        allowing any boolean expression that references account fields or other
        instruction parameters.

        This adds the Anchor #[account(constraint = <expr>)] constraint.

        Example:
            my_account.constraint(my_account.authority == signer.key())

        @param expr: A boolean expression that must be True for validation to pass.
        @returns: Self for method chaining.
        """

    def rent_exempt(self, mode: str) -> 'AccountWithKey':
        """
        Control rent exemption enforcement for this account.

        This adds the Anchor #[account(rent_exempt = <mode>)] constraint which controls
        whether the account must be rent-exempt. By default, Anchor enforces rent exemption
        for initialized accounts.

        @param mode: Either "skip" to skip rent-exempt check, or "enforce" to require it.
        @returns: Self for method chaining.
        """

    def seeds(self, seeds: List[Any]) -> 'AccountWithKey':
        """
        Add PDA seeds constraint to verify this existing account.

        This adds the Anchor #[account(seeds = [...], bump)] constraint which verifies
        at runtime that the account is the correct PDA derived from the given seeds.
        Use this to verify an existing PDA account without initializing it.

        Example:
            @instruction
            def verify_pda(user: Signer, config: Config):
                config.seeds([b"config", user.key()])  # Verify it's the right PDA

        This generates:
            #[account(
                mut,
                seeds = [b"config", user.key().as_ref()],
                bump
            )]
            pub config: Account<'info, Config>,

        @param seeds: A list of seed values (bytes, pubkeys, integers, etc.) used to derive the PDA.
        @returns: Self for method chaining.
        """

    def bump(self, value: u8) -> 'AccountWithKey':
        """
        Specify an explicit bump value for PDA derivation.

        Use this when you have stored the bump and want to use it directly
        instead of having Anchor derive it. This is more efficient as it
        avoids the runtime search for a valid bump.

        This should be used in conjunction with seeds() to specify the
        bump value for PDA verification.

        Example:
            @instruction
            def verify_pda(user: Signer, config: Config):
                # Use stored bump instead of deriving
                config.seeds([b"config", user.key()]).bump(config.stored_bump)

        This generates:
            #[account(
                mut,
                seeds = [b"config", user.key().as_ref()],
                bump = config.stored_bump
            )]
            pub config: Account<'info, Config>,

        @param value: The bump value (u8) to use for PDA derivation.
        @returns: Self for method chaining.
        """

class Account(AccountWithKey):
    """User-defined Solana account."""

    def transfer_lamports(self, to: AccountWithKey, amount: u64):
        """
        Transfer some SOL directly to another account. Since this account is program-owned, this transfer does not require a CPI.

        @param to: The recipient Solana account.
        @param amount: The amount (in lamports, not SOL) to transfer.
        """

class Event:
    """Anchor event that clients can listen for"""

    def emit(self):
        """
        Emit the event to the blockchain
        """

class Signer(AccountWithKey):
    """Instruction signer."""

    def transfer_lamports(self, to: AccountWithKey, amount: u64):
        """
        Transfer some SOL directly to another account. Unlike using transfer_lamports from a program account, this transfer will require a CPI.

        @param to: The recipient Solana account.
        @param amount: The amount (in lamports, not SOL) to transfer.
        """

class Empty(Generic[T]):
    """An account that needs to be initialized."""

    def init(self, payer: Signer, seeds: List[Any] = None, mint: 'TokenMint' = None, decimals: u8 = None, authority: AccountWithKey = None, associated: bool = False, space: u64 = None, padding: u64 = None)  -> T:
        """
        Initialize the account.
        
        @param payer: The account that will pay for the rent cost of the initialized account. Must be an instruction signer.
        @param seeds: A list of parameters to uniquely identify this account among all accounts created by your program. These may be string literals, other accounts, integers, or lists of bytes.
        @param mint: If initializing a TokenAccount, this is the mint that the account belongs to.
        @param decimals: If initializing a TokenMint, this is the number of decimals the new token has.
        @param authority: If initializing a TokenAccount/TokenMint, this is the account that has authority over the account.
        @param associated: If initializing an associated token account, must be set to true.
        @param space: If initializing a program account, you can use this to overwrite Seahorse's calculation of the account size.
        @param padding: If initializing a program account, you can use this to add extra space to Seahorse's calculation of the account size.
        @returns: The new, initialized account. All of the data in this account will be set to 0, bytewise.
        """

    def bump(self) -> u8:
        """
        Get this account's bump, needed if you want to use this account to sign CPI calls.
        
        If you've initialized an account without seeds, then a bump will not have been calculated. This will result in a runtime error when you try to access it.
        """

    def key(self) -> Pubkey:
        """Get this account's key."""

class CpiAccount:
    """Account and metadata used for making arbitrary CPIs (via `Program.invoke`)."""

    def __init__(account: AccountWithKey, mut: bool = False, signer: bool = False, seeds: List[Any] = None) -> 'CpiAccount':
        """
        Create the CpiAccount.

        @param account: The account being passed to the CPI.
        @param mut: Whether this account needs to be mutable for the CPI - defaults to false.
        @param signer: Whether this account needs to be an instruction signer - defaults to false. Mutually exclusive with seeds, and should only really be true if account is a Signer.
        @param seeds: PDA signer seeds, if this account needs to sign the CPI. Mutually exclusive with signer.
        """

class Program(AccountWithKey):
    """Arbitrary program."""

    def invoke(self, accounts: List[CpiAccount], data: List[u8]):
        """
        Call this program in a cross-program invocation. Make sure you know what you're doing before you try using this - a poorly crafted data list could cost you real money.

        @param accounts: List of accounts being passed to the CPI - the program itself does not need to be in here.
        @param data: "Raw" list of bytes used to tell the program what to do, pass args, etc.
        """

class UncheckedAccount(AccountWithKey):
    """
    Raw account that has had no safety checks performed on it.
    
    The underlying Anchor code cannot guarantee anything about the account unless you check it in your instruction - not the type, not the data, not the program it came from. Use carefully.
    """

class Clock:
    """
    Solana's Clock sysvar.
    
    Consult Solana's reference to learn more. Information copied from https://docs.rs/solana-program/1.14.3/solana_program/clock/struct.Clock.html.
    """

    def slot(self) -> u64:
        """Get the current network/bank Slot."""

    def epoch_start_timestamp(self) -> i64:
        """Get the timestamp of the first Slot in this Epoch."""

    def epoch(self) -> u64:
        """Get the bank Epoch."""

    def leader_schedule_epoch(self) -> u64:
        """Get the future Epoch for which the leader schedule has most recently been calculated."""

    def unix_timestamp(self) -> i64:
        """
        Get the estimated current UNIX timestamp.
        
        Originally computed from genesis creation time and network time in slots (drifty); corrected using validator timestamp oracle as of timestamp_correction and timestamp_bounding features.
        """

class TokenAccount(AccountWithKey):
    """SPL token account."""

    def authority(self) -> Pubkey:
        """Get the owner of this token account."""

    def amount(self) -> u64:
        """Get the amount of token stored in this account."""

    def mint(self) -> Pubkey:
        """Get the mint that this token account corresponds to."""

    def transfer(self, authority: AccountWithKey, to: 'TokenAccount', amount: u64, signer: List[Any] = None):
        """
        Transfer funds from this SPL token account to another.

        @param authority: The account that owns this TokenAccount. Must be an instruction signer or the account given by the `signer` param.
        @param to: The recipient TokenAccount.
        @param amount: How much (in *native* token units) to transfer.
        @param signer: (Optional) seeds for the signature of a PDA.
        """

    def token_mint(self, mint: 'TokenMint') -> 'TokenAccount':
        """
        Add a token::mint check constraint to verify this token account's mint.

        This adds the Anchor #[account(token::mint = <mint>)] constraint which verifies
        at runtime that the token account's mint matches the expected mint account.

        @param mint: The expected TokenMint for this token account.
        @returns: Self for method chaining.
        """

    def token_authority(self, authority: Pubkey) -> 'TokenAccount':
        """
        Add a token::authority check constraint to verify this token account's authority.

        This adds the Anchor #[account(token::authority = <authority>)] constraint which verifies
        at runtime that the token account's owner matches the expected pubkey.

        @param authority: The expected authority (owner) pubkey for this token account.
        @returns: Self for method chaining.
        """

    def token_program(self, program: 'Program') -> 'TokenAccount':
        """
        Add a token::token_program check constraint to verify the token program.

        This adds the Anchor #[account(token::token_program = <program>)] constraint which verifies
        at runtime that the token account uses the expected token program (e.g., SPL Token or Token-2022).

        @param program: The expected token program.
        @returns: Self for method chaining.
        """

class TokenMint(AccountWithKey):
    """SPL token mint."""

    def authority(self) -> Pubkey:
        """Get the owner of this token mint."""

    def freeze_authority(self) -> Pubkey:
        """Get the freeze authority of this token mint."""

    def decimals(self) -> u8:
        """Get the number of decimals for this token."""

    def supply(self) -> u64:
        """Get the amount of this token that exists."""

    def mint(self, authority: AccountWithKey, to: TokenAccount, amount: u64, signer: List[Any] = None):
        """
        Mint new tokens to a token account.

        @param authority: The account that owns this TokenMint. Must be an instruction signer or the account given by the `signer` param.
        @param to: The recipient TokenAccount.
        @param amount: How much (in *native* token units) to mint.
        @param signer: (Optional) seeds for the signature of a PDA.
        """

    def burn(self, authority: AccountWithKey, holder: TokenAccount, amount: u64, signer: List[Any] = None):
        """
        Burn tokens from a token account.

        @param authority: The account that owns the `holder` TokenAccount. Must be an instruction signer or the account given by the `signer` param.
        @param holder: The TokenAccount to burn from.
        @param amount: How much (in *native* token units) to burn.
        @param signer: (Optional) seeds for the signature of a PDA.
        """

    def mint_decimals(self, decimals: u8) -> 'TokenMint':
        """
        Add a mint::decimals check constraint to verify this mint's decimals.

        This adds the Anchor #[account(mint::decimals = <decimals>)] constraint which verifies
        at runtime that the mint's decimals match the expected value.

        @param decimals: The expected number of decimals for this mint.
        @returns: Self for method chaining.
        """

    def mint_authority(self, authority: Pubkey) -> 'TokenMint':
        """
        Add a mint::authority check constraint to verify this mint's authority.

        This adds the Anchor #[account(mint::authority = <authority>)] constraint which verifies
        at runtime that the mint's mint_authority matches the expected pubkey.

        @param authority: The expected mint authority pubkey.
        @returns: Self for method chaining.
        """

    def mint_freeze_authority(self, authority: Pubkey) -> 'TokenMint':
        """
        Add a mint::freeze_authority check constraint to verify this mint's freeze authority.

        This adds the Anchor #[account(mint::freeze_authority = <authority>)] constraint which verifies
        at runtime that the mint's freeze_authority matches the expected pubkey.

        @param authority: The expected freeze authority pubkey.
        @returns: Self for method chaining.
        """

    def mint_token_program(self, program: 'Program') -> 'TokenMint':
        """
        Add a mint::token_program check constraint to verify the token program.

        This adds the Anchor #[account(mint::token_program = <program>)] constraint which verifies
        at runtime that the mint uses the expected token program (e.g., SPL Token or Token-2022).

        @param program: The expected token program.
        @returns: Self for method chaining.
        """


# ================
# Helper functions
# ================

def declare_id(id: str):
    """Inform Anchor what this program's ID is.

    @param id: The program's ID, generated by Anchor in /target/idl/<program>.json. This must be copied-pasted straight from there as a string literal.
    """

def instruction(function: Callable[..., None]) -> Callable[..., None]:
    """Decorator to turn a function into a program instruction."""

def dataclass(function: Callable[..., None]) -> Callable[..., None]:
    """Decorator to create an automatic default class constructor."""

def int_bytes(n: Any, be: bool = False) -> List[u8]:
    """
    Convenience method to turn an integer type into a little-endian (by default) list of bytes.
    
    @param n: The integer you wish to convert to bytes.
    @param be: Whether you want the conversion to be big-endian - defaults to false.
    """

def size(ob: str) -> u64:
    """
    Get the size of an object in bytes.
    Currently this is only supported for strings.
    
    @param ob: The object to get the size of.
    """
