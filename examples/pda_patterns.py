# pda_patterns
# Built with Seahorse v0.1.0
#
# Demonstrates various PDA derivation patterns:
# - Single seed: global config
# - User-derived: user profile with owner key
# - Multiple seeds: data record with category + id
# - Multiple seeds: vault with owner + id

from seahorse.prelude import *

declare_id('DxpKiPEUYHy6NKsnEMRvRKtC3ncrtP8epGt5KKS59Xzh')


class GlobalConfig(Account):
    admin: Pubkey
    counter: u64
    bump: u8


class UserProfile(Account):
    owner: Pubkey
    name: str
    visits: u64
    bump: u8


class DataRecord(Account):
    creator: Pubkey
    category: str
    record_id: u64
    data: str
    bump: u8


class Vault(Account):
    owner: Pubkey
    vault_id: u64
    balance: u64
    bump: u8


# ============================================
# Single seed pattern: Global Config
# PDA: ["global_config"]
# ============================================

@instruction
def init_global_config(admin: Signer, config: Empty[GlobalConfig]):
    bump = config.bump()
    config = config.init(payer=admin, seeds=['global_config'])
    config.admin = admin.key()
    config.counter = 0
    config.bump = bump


@instruction
def update_global_config(admin: Signer, config: GlobalConfig):
    assert admin.key() == config.admin, 'Unauthorized'
    config.counter += 1


# ============================================
# User-derived PDA pattern: User Profile
# PDA: ["user_profile", user.key()]
# ============================================

@instruction
def init_user_profile(user: Signer, profile: Empty[UserProfile], name: str):
    bump = profile.bump()
    profile = profile.init(payer=user, seeds=['user_profile', user])
    profile.owner = user.key()
    profile.name = name
    profile.visits = 0
    profile.bump = bump


@instruction
def visit_profile(user: Signer, profile: UserProfile):
    # Note: In Seahorse, the PDA constraint is derived from seeds at init time
    # Authorization is checked via the owner field
    assert user.key() == profile.owner, 'Unauthorized'
    profile.visits += 1


# ============================================
# Multiple seeds pattern: Data Record
# PDA: ["data_record", category, record_id]
# ============================================

@instruction
def init_data_record(
    creator: Signer,
    record: Empty[DataRecord],
    category: str,
    record_id: u64,
    data: str
):
    bump = record.bump()
    record = record.init(payer=creator, seeds=['data_record', category, record_id])
    record.creator = creator.key()
    record.category = category
    record.record_id = record_id
    record.data = data
    record.bump = bump


@instruction
def update_data_record(
    creator: Signer,
    record: DataRecord,
    data: str
):
    assert creator.key() == record.creator, 'Unauthorized'
    record.data = data


# ============================================
# Multiple seeds pattern: Vault
# PDA: ["vault", owner.key(), vault_id]
# ============================================

@instruction
def init_vault(owner: Signer, vault: Empty[Vault], vault_id: u64):
    bump = vault.bump()
    vault = vault.init(payer=owner, seeds=['vault', owner, vault_id])
    vault.owner = owner.key()
    vault.vault_id = vault_id
    vault.balance = 0
    vault.bump = bump


@instruction
def deposit_to_vault(owner: Signer, vault: Vault, amount: u64):
    assert owner.key() == vault.owner, 'Unauthorized'
    vault.balance += amount
