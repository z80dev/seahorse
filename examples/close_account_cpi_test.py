# close_account_cpi_test.py
# Test file for the close_account CPI implementation

from seahorse.prelude import *

declare_id('Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS')

@instruction
def close_token_account(
    authority: Signer,
    token_account: TokenAccount,
    destination: Signer
):
    # Close the token account and send remaining lamports to destination
    token_account.close_account(
        authority=authority,
        destination=destination
    )
