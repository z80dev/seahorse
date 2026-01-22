# has_one_test.py
# Test file for the has_one constraint implementation

from seahorse.prelude import *

declare_id('HAS1111111111111111111111111111111111111111')

class Vault(Account):
    owner: Pubkey
    authority: Pubkey
    data: u64

@instruction
def create_vault(owner: Signer, vault: Empty[Vault]):
    vault = vault.init(payer=owner, seeds=['vault', owner])
    vault.owner = owner.key()
    vault.authority = owner.key()
    vault.data = 0

@instruction
def update_vault(authority: Signer, vault: Vault):
    # Use has_one to verify authority matches vault.authority
    vault.has_one(authority)
    vault.data = vault.data + 1

@instruction
def transfer_authority(
    authority: Signer,
    new_authority: Signer,
    vault: Vault
):
    # has_one verifies vault.authority == authority.key()
    vault.has_one(authority)
    vault.authority = new_authority.key()
