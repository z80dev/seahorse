use anchor_lang::prelude::*;

declare_id!("He11oWor1d111111111111111111111111111111111");

#[program]
pub mod hello_world_anchor {
    use super::*;

    pub fn say_hello(_ctx: Context<SayHello>) -> Result<()> {
        msg!("Hello world, from Solana smart contract");
        Ok(())
    }
}

#[derive(Accounts)]
pub struct SayHello<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
}
