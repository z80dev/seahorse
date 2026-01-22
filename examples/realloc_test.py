# realloc_test.py
# Test file for the realloc constraint implementation

from seahorse.prelude import *

declare_id('Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS')

class DynamicData(Account):
    owner: Pubkey
    size: u64
    data: Array[u8, 100]

@instruction
def initialize(owner: Signer, data: Empty[DynamicData]):
    data = data.init(payer=owner, seeds=['data', owner])
    data.owner = owner.key()
    data.size = 100

@instruction
def grow_data(owner: Signer, data: DynamicData):
    # Use realloc to grow the account
    data.realloc(200, payer=owner, zero=True)
    data.size = 200

@instruction
def shrink_data(owner: Signer, data: DynamicData):
    # Use realloc to shrink the account (payer receives excess rent)
    # Note: minimum size is 148 bytes (8 disc + 32 owner + 8 size + 100 data)
    # Shrink from 200 to 150 (just above minimum)
    data.realloc(150, payer=owner, zero=False)
    data.size = 150
