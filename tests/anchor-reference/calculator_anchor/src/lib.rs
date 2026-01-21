use anchor_lang::prelude::*;

// Use same program ID as Seahorse calculator for parity testing
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod calculator_anchor {
    use super::*;

    pub fn init_calculator(ctx: Context<InitCalculator>) -> Result<()> {
        let calculator = &mut ctx.accounts.calculator;
        calculator.owner = ctx.accounts.owner.key();
        calculator.display = 0;
        msg!("{} is initializing calculator {}", ctx.accounts.owner.key(), ctx.accounts.calculator.key());
        Ok(())
    }

    pub fn reset_calculator(ctx: Context<UpdateCalculator>) -> Result<()> {
        let calculator = &mut ctx.accounts.calculator;
        msg!("{} is resetting a calculator {}", ctx.accounts.owner.key(), calculator.key());
        calculator.display = 0;
        Ok(())
    }

    pub fn do_operation(ctx: Context<UpdateCalculator>, op: Operation, num: i64) -> Result<()> {
        let calculator = &mut ctx.accounts.calculator;

        match op {
            Operation::Add => {
                calculator.display = calculator.display.checked_add(num).ok_or(ErrorCode::Overflow)?;
            }
            Operation::Sub => {
                calculator.display = calculator.display.checked_sub(num).ok_or(ErrorCode::Underflow)?;
            }
            Operation::Mul => {
                calculator.display = calculator.display.checked_mul(num).ok_or(ErrorCode::Overflow)?;
            }
            Operation::Div => {
                require!(num != 0, ErrorCode::DivisionByZero);
                calculator.display = calculator.display / num;
            }
        }

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitCalculator<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init,
        payer = owner,
        space = 8 + Calculator::INIT_SPACE,
        seeds = [b"Calculator", owner.key().as_ref()],
        bump
    )]
    pub calculator: Account<'info, Calculator>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateCalculator<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [b"Calculator", owner.key().as_ref()],
        bump,
        has_one = owner @ ErrorCode::Unauthorized
    )]
    pub calculator: Account<'info, Calculator>,
}

#[account]
#[derive(InitSpace)]
pub struct Calculator {
    pub owner: Pubkey,
    pub display: i64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Add,
    Sub,
    Mul,
    Div,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Arithmetic underflow")]
    Underflow,
    #[msg("Division by zero")]
    DivisionByZero,
    #[msg("This is not your calculator!")]
    Unauthorized,
}
