# errors
# Built with Seahorse v0.1.0
#
# Demonstrates custom error definitions and handling patterns:
# - Custom error messages with assert statements
# - Validation checks with descriptive error messages
# - Authorization checks with error handling
# - Numeric validation (range checks, overflow prevention)
# - String length validation
# - State validation errors
#
# Key patterns:
# - Seahorse uses Python assert with error messages
# - Compiler translates assert to Anchor's require! macro
# - Error messages are embedded in the compiled Rust code
#
# Design note: Seahorse doesn't support custom error enums directly.
# Instead, errors are defined via assert statements with message strings.
# These get compiled to: require!(condition, CustomError::SomeVariant)
# where the error variant is auto-generated from the message.

from seahorse.prelude import *

declare_id('ErrDemo111111111111111111111111111111111111')


class ErrorDemo(Account):
    # Authority who can modify this account
    authority: Pubkey
    # Current value for numeric tests
    value: u64
    # Counter for operations
    operation_count: u64
    # Status flag
    is_active: bool
    # Name field for string validation
    name: str
    # Maximum allowed value (for range checking)
    max_value: u64
    # PDA bump seed
    bump: u8


class RoleRegistry(Account):
    # Account authority (admin)
    admin: Pubkey
    # Registered operator (secondary role)
    operator: Pubkey
    # Whether operator is set
    has_operator: bool
    # Role registry ID
    registry_id: u64
    # PDA bump seed
    bump: u8


# =============================================================================
# INITIALIZE - Basic initialization
# =============================================================================

@instruction
def initialize(
    authority: Signer,
    error_demo: Empty[ErrorDemo],
    max_value: u64,
    name: str
):
    """Initialize an ErrorDemo account with validation checks."""
    # Input validation errors
    assert max_value > 0, 'Max value must be greater than zero'
    assert max_value <= 1_000_000, 'Max value exceeds maximum allowed (1,000,000)'
    assert len(name) > 0, 'Name cannot be empty'
    assert len(name) <= 32, 'Name exceeds maximum length of 32 characters'

    # Get bump before init
    demo_bump = error_demo.bump()

    # Initialize the account
    error_demo = error_demo.init(
        payer=authority,
        seeds=['error_demo', authority],
        padding=100  # Extra space for name string
    )

    error_demo.authority = authority.key()
    error_demo.value = 0
    error_demo.operation_count = 0
    error_demo.is_active = True
    error_demo.name = name
    error_demo.max_value = max_value
    error_demo.bump = demo_bump

    print(f'ErrorDemo initialized: authority={authority.key()}, max_value={max_value}')


# =============================================================================
# NUMERIC VALIDATION - Range and overflow checks
# =============================================================================

@instruction
def set_value(
    authority: Signer,
    error_demo: ErrorDemo,
    new_value: u64
):
    """Set value with authorization and range validation."""
    # Authorization error
    assert authority.key() == error_demo.authority, 'Unauthorized: only authority can set value'

    # State validation error
    assert error_demo.is_active, 'Account is not active'

    # Range validation error
    assert new_value <= error_demo.max_value, 'Value exceeds maximum allowed'

    error_demo.value = new_value
    error_demo.operation_count += 1

    print(f'Value set to {new_value}')


@instruction
def increment_value(
    authority: Signer,
    error_demo: ErrorDemo,
    amount: u64
):
    """Increment value with overflow prevention."""
    # Authorization check
    assert authority.key() == error_demo.authority, 'Unauthorized: only authority can increment'

    # State validation
    assert error_demo.is_active, 'Account is not active'

    # Amount validation
    assert amount > 0, 'Increment amount must be greater than zero'

    # Overflow prevention
    new_value = error_demo.value + amount
    assert new_value >= error_demo.value, 'Overflow: increment would overflow'
    assert new_value <= error_demo.max_value, 'Value would exceed maximum allowed'

    error_demo.value = new_value
    error_demo.operation_count += 1

    print(f'Value incremented by {amount} to {new_value}')


@instruction
def decrement_value(
    authority: Signer,
    error_demo: ErrorDemo,
    amount: u64
):
    """Decrement value with underflow prevention."""
    # Authorization check
    assert authority.key() == error_demo.authority, 'Unauthorized: only authority can decrement'

    # State validation
    assert error_demo.is_active, 'Account is not active'

    # Amount validation
    assert amount > 0, 'Decrement amount must be greater than zero'

    # Underflow prevention
    assert error_demo.value >= amount, 'Underflow: value would go below zero'

    error_demo.value = error_demo.value - amount
    error_demo.operation_count += 1

    print(f'Value decremented by {amount} to {error_demo.value}')


# =============================================================================
# STRING VALIDATION
# =============================================================================

@instruction
def update_name(
    authority: Signer,
    error_demo: ErrorDemo,
    new_name: str
):
    """Update name with string validation."""
    # Authorization check
    assert authority.key() == error_demo.authority, 'Unauthorized: only authority can update name'

    # State validation
    assert error_demo.is_active, 'Account is not active'

    # String validation
    assert len(new_name) > 0, 'Name cannot be empty'
    assert len(new_name) <= 32, 'Name exceeds maximum length of 32 characters'

    # Note: print before assignment to avoid moved value issue in generated Rust
    print(f'Name updated to: {new_name}')

    error_demo.name = new_name
    error_demo.operation_count += 1


# =============================================================================
# STATE TRANSITION ERRORS
# =============================================================================

@instruction
def deactivate(
    authority: Signer,
    error_demo: ErrorDemo
):
    """Deactivate the account (can only be done once when active)."""
    # Authorization check
    assert authority.key() == error_demo.authority, 'Unauthorized: only authority can deactivate'

    # State transition validation
    assert error_demo.is_active, 'Account is already deactivated'

    error_demo.is_active = False
    error_demo.operation_count += 1

    print('Account deactivated')


@instruction
def reactivate(
    authority: Signer,
    error_demo: ErrorDemo
):
    """Reactivate the account (can only be done when inactive)."""
    # Authorization check
    assert authority.key() == error_demo.authority, 'Unauthorized: only authority can reactivate'

    # State transition validation
    assert not error_demo.is_active, 'Account is already active'

    error_demo.is_active = True
    error_demo.operation_count += 1

    print('Account reactivated')


# =============================================================================
# ROLE-BASED ACCESS CONTROL ERRORS
# =============================================================================

@instruction
def initialize_role_registry(
    admin: Signer,
    registry: Empty[RoleRegistry],
    registry_id: u64
):
    """Initialize a role registry with admin."""
    # Get bump before init
    registry_bump = registry.bump()

    registry = registry.init(
        payer=admin,
        seeds=['role_registry', registry_id]
    )

    registry.admin = admin.key()
    registry.operator = admin.key()  # Default operator to admin
    registry.has_operator = False
    registry.registry_id = registry_id
    registry.bump = registry_bump

    print(f'RoleRegistry {registry_id} initialized with admin {admin.key()}')


@instruction
def set_operator(
    admin: Signer,
    registry: RoleRegistry,
    new_operator: Pubkey
):
    """Set operator role (admin only)."""
    # Admin-only check
    assert admin.key() == registry.admin, 'Unauthorized: only admin can set operator'

    registry.operator = new_operator
    registry.has_operator = True

    print(f'Operator set to {new_operator}')


@instruction
def admin_only_action(
    caller: Signer,
    registry: RoleRegistry
):
    """Action that requires admin role."""
    # Admin-only check
    assert caller.key() == registry.admin, 'Unauthorized: admin role required'

    print(f'Admin action executed by {caller.key()}')


@instruction
def operator_action(
    caller: Signer,
    registry: RoleRegistry
):
    """Action that requires operator or admin role."""
    # Must have operator set
    assert registry.has_operator, 'No operator has been set'

    # Operator or admin check
    is_admin = caller.key() == registry.admin
    is_operator = caller.key() == registry.operator
    assert is_admin or is_operator, 'Unauthorized: operator or admin role required'

    print(f'Operator action executed by {caller.key()}')


# =============================================================================
# MULTIPLE VALIDATION ERRORS IN SEQUENCE
# =============================================================================

@instruction
def complex_validation(
    authority: Signer,
    error_demo: ErrorDemo,
    new_value: u64,
    new_name: str,
    require_high_value: bool
):
    """Demonstrate multiple validation checks in sequence."""
    # Authorization (first check)
    assert authority.key() == error_demo.authority, 'Unauthorized: only authority can perform complex validation'

    # State check (second check)
    assert error_demo.is_active, 'Account is not active'

    # Numeric validations
    assert new_value > 0, 'Value must be greater than zero'
    assert new_value <= error_demo.max_value, 'Value exceeds maximum allowed'

    # Conditional validation
    # Note: Using integer division to avoid float issues in generated Rust
    half_max = error_demo.max_value // 2
    if require_high_value:
        assert new_value >= half_max, 'Value must be at least half of max when high value required'

    # String validations
    assert len(new_name) > 0, 'Name cannot be empty'
    assert len(new_name) <= 32, 'Name exceeds maximum length of 32 characters'

    # Note: print before assignment to avoid moved value issue in generated Rust
    print(f'Complex validation passed: value={new_value}, name={new_name}')

    # Apply changes
    error_demo.value = new_value
    error_demo.name = new_name
    error_demo.operation_count += 1


# =============================================================================
# READ-ONLY INFO
# =============================================================================

@instruction
def get_demo_info(error_demo: ErrorDemo):
    """Get ErrorDemo info (read-only, for logging)."""
    print(f'Authority: {error_demo.authority}')
    print(f'Value: {error_demo.value}')
    print(f'Max Value: {error_demo.max_value}')
    print(f'Name: {error_demo.name}')
    print(f'Is Active: {error_demo.is_active}')
    print(f'Operation Count: {error_demo.operation_count}')
