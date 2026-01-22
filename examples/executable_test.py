# executable_test.py
# Test file for the executable constraint implementation

from seahorse.prelude import *

declare_id('EXEC111111111111111111111111111111111111111')

@instruction
def verify_program(
    user: Signer,
    program_to_check: UncheckedAccount
):
    """
    Verify that an account is executable (a program).
    The executable constraint will fail at runtime if the account is not a program.
    """
    # Apply the executable constraint to verify this is a program
    program_to_check.executable()

@instruction
def verify_multiple_programs(
    user: Signer,
    first_program: UncheckedAccount,
    second_program: UncheckedAccount
):
    """
    Verify multiple accounts are executable programs.
    """
    first_program.executable()
    second_program.executable()
