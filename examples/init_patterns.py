# init_patterns
# Built with Seahorse v0.1.0
#
# Demonstrates account initialization patterns and space calculation:
# - Basic init with primitive types (u8, u16, u32, u64, i64)
# - Init with dynamic strings requiring padding
# - Init with fixed-size arrays
# - Init with nested structs
# - Rent exemption is automatic

from seahorse.prelude import *

declare_id('InitPtrn1111111111111111111111111111111111')


# ============================================
# Nested struct for complex data
# ============================================

class Stats:
    count: u64
    total: u64
    average: u64

    def __init__(self, count: u64, total: u64, average: u64):
        self.count = count
        self.total = total
        self.average = average


# ============================================
# Account definitions with different field types
# ============================================

# Simple data with various primitive types.
# Space: owner(32) + u8(1) + u16(2) + u32(4) + u64(8) + i64(8) + bump(1) = 56 bytes
class SimpleData(Account):
    owner: Pubkey
    value_u8: u8
    value_u16: u16
    value_u32: u32
    value_u64: u64
    value_i64: i64
    bump: u8


# String data demonstrating variable-length field sizing.
# Space: owner(32) + name(4+32) + description(4+128) + bump(1) = 201 bytes
# Note: Seahorse uses padding parameter for variable data
class StringData(Account):
    owner: Pubkey
    name: str
    description: str
    bump: u8


# Array data demonstrating fixed-size array sizing.
# Space: owner(32) + values([u64; 4]=32) + flags([bool; 8]=8) + bump(1) = 73 bytes
class ArrayData(Account):
    owner: Pubkey
    values: Array[u64, 4]
    flags: Array[bool, 8]
    bump: u8


# Complex data with nested struct.
# Space: owner(32) + Stats(24) + is_active(1) + bump(1) = 58 bytes
class ComplexData(Account):
    owner: Pubkey
    stats: Stats
    is_active: bool
    bump: u8


# ============================================
# Instructions: SimpleData initialization
# ============================================

# Initialize a simple data account with primitive types.
# Demonstrates basic space calculation for primitives.
@instruction
def init_simple(owner: Signer, data: Empty[SimpleData], value_u8: u8, value_u64: u64):
    bump = data.bump()
    data = data.init(payer=owner, seeds=['simple', owner])
    data.owner = owner.key()
    data.value_u8 = value_u8
    data.value_u16 = 0
    data.value_u32 = 0
    data.value_u64 = value_u64
    data.value_i64 = 0
    data.bump = bump


# Update simple data values.
@instruction
def update_simple(
    owner: Signer,
    data: SimpleData,
    value_u8: u8,
    value_u16: u16,
    value_u32: u32,
    value_u64: u64,
    value_i64: i64
):
    assert owner.key() == data.owner, 'Unauthorized'
    data.value_u8 = value_u8
    data.value_u16 = value_u16
    data.value_u32 = value_u32
    data.value_u64 = value_u64
    data.value_i64 = value_i64


# ============================================
# Instructions: StringData initialization
# ============================================

# Initialize a data account with dynamic strings.
# Demonstrates padding for variable-length data.
# Padding: 200 bytes for name(32) + description(128) + overhead
@instruction
def init_with_string(owner: Signer, data: Empty[StringData], name: str, description: str):
    bump = data.bump()
    # Use padding for variable-length string data
    data = data.init(payer=owner, seeds=['string_data', owner], padding=200)
    data.owner = owner.key()
    data.name = name
    data.description = description
    data.bump = bump


# Update string data.
@instruction
def update_string(owner: Signer, data: StringData, name: str, description: str):
    assert owner.key() == data.owner, 'Unauthorized'
    data.name = name
    data.description = description


# ============================================
# Instructions: ArrayData initialization
# ============================================

# Initialize a data account with fixed-size arrays.
# Demonstrates array space calculation.
# Arrays are zero-initialized by default.
@instruction
def init_with_array(owner: Signer, data: Empty[ArrayData]):
    bump = data.bump()
    data = data.init(payer=owner, seeds=['array_data', owner])
    data.owner = owner.key()
    # Arrays are zero-initialized by default, set individual elements if needed
    data.values[0] = 0
    data.values[1] = 0
    data.values[2] = 0
    data.values[3] = 0
    data.flags[0] = False
    data.flags[1] = False
    data.flags[2] = False
    data.flags[3] = False
    data.flags[4] = False
    data.flags[5] = False
    data.flags[6] = False
    data.flags[7] = False
    data.bump = bump


# Update array values.
# Note: Seahorse doesn't support passing arrays as instruction args directly,
# so we pass individual values.
@instruction
def update_array(
    owner: Signer,
    data: ArrayData,
    v0: u64, v1: u64, v2: u64, v3: u64,
    f0: bool, f1: bool, f2: bool, f3: bool, f4: bool, f5: bool, f6: bool, f7: bool
):
    assert owner.key() == data.owner, 'Unauthorized'
    data.values[0] = v0
    data.values[1] = v1
    data.values[2] = v2
    data.values[3] = v3
    data.flags[0] = f0
    data.flags[1] = f1
    data.flags[2] = f2
    data.flags[3] = f3
    data.flags[4] = f4
    data.flags[5] = f5
    data.flags[6] = f6
    data.flags[7] = f7


# ============================================
# Instructions: ComplexData initialization
# ============================================

# Initialize a complex nested account.
# Demonstrates nested struct space calculation.
@instruction
def init_complex(owner: Signer, data: Empty[ComplexData], initial_count: u64):
    bump = data.bump()
    data = data.init(payer=owner, seeds=['complex', owner])
    data.owner = owner.key()
    data.stats = Stats(initial_count, u64(0), u64(0))
    data.is_active = True
    data.bump = bump


# Update complex data stats.
@instruction
def update_complex(owner: Signer, data: ComplexData, count: u64, total: u64):
    assert owner.key() == data.owner, 'Unauthorized'
    data.stats.count = count
    data.stats.total = total
    # Calculate average (safe division)
    if count > 0:
        data.stats.average = total // count
    else:
        data.stats.average = 0


# Toggle complex data active state.
@instruction
def toggle_active(owner: Signer, data: ComplexData):
    assert owner.key() == data.owner, 'Unauthorized'
    data.is_active = not data.is_active
