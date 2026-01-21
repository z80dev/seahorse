# counter
# Built with Seahorse v0.1.0
#
# Simple counter demonstrating basic account creation, mutation, and PDA seeds.

from seahorse.prelude import *

declare_id('CntrQd1yLLfEMvj47u3qHxq5xW3jcfC2E51h4jRmpump')


class Counter(Account):
    count: u64
    authority: Pubkey


@instruction
def initialize(authority: Signer, counter: Empty[Counter]):
    counter = counter.init(payer=authority, seeds=['counter', authority])
    counter.count = 0
    counter.authority = authority.key()


@instruction
def increment(authority: Signer, counter: Counter):
    assert authority.key() == counter.authority, 'Unauthorized'
    counter.count += 1


@instruction
def decrement(authority: Signer, counter: Counter):
    assert authority.key() == counter.authority, 'Unauthorized'
    assert counter.count > 0, 'Counter underflow'
    counter.count -= 1


@instruction
def set_value(authority: Signer, counter: Counter, value: u64):
    assert authority.key() == counter.authority, 'Unauthorized'
    counter.count = value
