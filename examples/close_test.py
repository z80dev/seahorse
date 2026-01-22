# close_test.py
# Test file for the close constraint implementation

from seahorse.prelude import *

declare_id('CLoSE11111111111111111111111111111111111111')

class Vault(Account):
    owner: Pubkey
    data: u64

@instruction
def create_vault(owner: Signer, vault: Empty[Vault]):
    vault = vault.init(payer=owner, seeds=['vault', owner])
    vault.owner = owner.key()
    vault.data = 0

@instruction
def close_vault(owner: Signer, vault: Vault):
    # Close the vault account and return rent to owner
    vault.close(owner)
