# init_if_needed_test.py
# Test file for the init_if_needed constraint implementation

from seahorse.prelude import *

declare_id('Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS')

class Counter(Account):
    owner: Pubkey
    count: u64

@instruction
def create_or_update_counter(owner: Signer, counter: Empty[Counter]):
    # init_if_needed: initializes account if it doesn't exist,
    # otherwise uses the existing account
    counter = counter.init_if_needed(payer=owner, seeds=['counter', owner])
    counter.owner = owner.key()
    counter.count = counter.count + 1
