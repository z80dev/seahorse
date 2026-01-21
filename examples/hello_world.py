# hello_world
# Built with Seahorse v0.1.0
#
# Minimal Hello World program demonstrating program logging.

from seahorse.prelude import *

declare_id('He11oWor1d111111111111111111111111111111111')


@instruction
def say_hello(signer: Signer):
    print('Hello world, from Solana smart contract')
