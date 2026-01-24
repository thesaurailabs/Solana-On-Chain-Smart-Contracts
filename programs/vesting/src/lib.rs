use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};
use chrono::prelude::*;
use solana_security_txt::security_txt;

security_txt! {
    name: "Vesting Smart Contract",
    project_url: "https://saurs.ai",
    contacts: "email:reachout@saurs.ai",
    preferred_languages: "en",
    source_code: "https://github.com/Arpit098/Vesting-Verify",
    policy: "https://saurs.ai/buynow",
    acknowledgements: "The SaurAI Labs"
}

declare_id!("CPZu31rBT7cWhWbwath7ZgxFGCsJbYyDX3tD9jKs1c4h");
const ADMIN: Pubkey = pubkey!("3c1gFBMmZFrDTgUz2HH8yhhbfqibdwfK14QtHRiQLYE1");

#[program]
pub mod vesting {
    use super::*;

    pub fn create_vesting_account(
        ctx: Context<CreateVestingAccount>,
        reserve_type: String,
    ) -> Result<()> {
        require!(
            ctx.accounts.signer.key() == ADMIN,
            ErrorCode::AccessDenied
        );
        let vesting_account = &mut ctx.accounts.vesting_account;
        vesting_account.owner = ctx.accounts.signer.key();
        vesting_account.mint = ctx.accounts.mint.key();
        vesting_account.treasury_token_account = ctx.accounts.treasury_token_account.key();
        vesting_account.reserve_type = reserve_type;
        vesting_account.bump = ctx.bumps.vesting_account;
        Ok(())
    }

    pub fn create_reserve(
        ctx: Context<CreateReserveAccount>,
        start_time: i64,
        end_time: i64,
        total_amount: i64,
        cliff_time: i64,
        monthly_claim: i64,
    ) -> Result<()> {
        require!(
            ctx.accounts.owner.key() == ADMIN,
            ErrorCode::AccessDenied
        );
        let reserve_account = &mut ctx.accounts.reserve_account;
        reserve_account.beneficiary = ctx.accounts.beneficiary.key();
        reserve_account.start_time = start_time;
        reserve_account.end_time = end_time;
        reserve_account.total_amount = total_amount;
        reserve_account.amount_withdrawn = 0;
        reserve_account.cliff_time = cliff_time;
        reserve_account.monthly_claim = monthly_claim;
        reserve_account.vesting_account = ctx.accounts.vesting_account.key();
        reserve_account.bump = ctx.bumps.reserve_account;

        let cpi_accounts = TransferChecked {
            mint: ctx.accounts.mint.to_account_info(),
            from: ctx.accounts.owner_token_account.to_account_info(),
            to: ctx.accounts.treasury_token_account.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        };

        let decimals = ctx.accounts.mint.decimals;
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        transfer_checked(cpi_ctx, total_amount.try_into().unwrap(), decimals)?;

        emit!(TokensLocked {
            amount: total_amount as u64,
            locked_until: start_time + cliff_time,
            unlock_amount_per_period: monthly_claim as u64,
            vesting_end_time: end_time,
            decimals: decimals,
        });

        Ok(())
    }

    pub fn claim_tokens(ctx: Context<ClaimTokens>, _reserve_type: String) -> Result<()> {
        let reserve_account = &mut ctx.accounts.reserve_account;
        let current_time = Clock::get()?.unix_timestamp;

        if current_time < reserve_account.start_time + reserve_account.cliff_time {
            return Err(ErrorCode::CliffPeriodNotEnded.into());
        }
        let v_start_time = reserve_account.start_time + reserve_account.cliff_time;

        let start_dt = DateTime::from_timestamp(v_start_time, 0).ok_or(ErrorCode::InvalidTime)?;
        let current_dt = DateTime::from_timestamp(current_time, 0).ok_or(ErrorCode::InvalidTime)?;
        
        let periods = months_elapsed(start_dt, current_dt) + 1;

        msg!("Debug Claim: CurrentTime: {}, VStartTime: {}", current_time, v_start_time);
        msg!("Debug Claim: MonthsElapsed: {}, Periods: {}", periods - 1, periods);
        
        let max_claimable = (periods as i64) * reserve_account.monthly_claim;
        let already_claimed = reserve_account.amount_withdrawn;
        let claimable = max_claimable
            .saturating_sub(already_claimed)
            .min(reserve_account.total_amount.saturating_sub(already_claimed));

        if claimable <= 0 {
            return Err(ErrorCode::ClaimNotAvailableYet.into());
        }
        reserve_account.amount_withdrawn = reserve_account.amount_withdrawn.saturating_add(claimable);

        let cpi_accounts = TransferChecked {
            mint: ctx.accounts.mint.to_account_info(),
            from: ctx.accounts.treasury_token_account.to_account_info(),
            to: ctx.accounts.beneficiary_token_account.to_account_info(),
            authority: ctx.accounts.vesting_account.to_account_info(),
        };

        let seeds = &[
            ctx.accounts.vesting_account.reserve_type.as_bytes(),
            &[ctx.accounts.vesting_account.bump],
        ];
        let signer_seeds = &[&seeds[..]];
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        transfer_checked(cpi_ctx, claimable as u64, ctx.accounts.mint.decimals)?;
        
        // Calculate correctly next claim time using calendar months
        let next_claim_time = if periods < 0 {
             v_start_time
        } else {
             // Use checked_add_months if available, or fallback to simple logic?
             // Chrono 0.4.23+ supports checked_add_months
             match start_dt.checked_add_months(chrono::Months::new(periods as u32)) {
                 Some(dt) => dt.timestamp(),
                 None => v_start_time + (periods * 30 * 24 * 60 * 60), // Fallback (should not happen)
             }
        };

        emit!(TokensClaimed {
            claimed_amount: claimable as u64,
            next_claim_timestamp: next_claim_time,
            decimals: ctx.accounts.mint.decimals,
        });

        Ok(())
    }
    pub fn close_reserve_account(ctx: Context<CloseReserveAccount>) -> Result<()> {
        let reserve_account = &ctx.accounts.reserve_account;
        let current_time = Clock::get()?.unix_timestamp;
    
        require!(
            current_time >= reserve_account.end_time,
            ErrorCode::VestingNotOver
        );
        require!(
            reserve_account.amount_withdrawn >= reserve_account.total_amount,
            ErrorCode::FundsRemaining
        );

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(reserve_type: String)]
pub struct CreateVestingAccount<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        space = 8 + VestingAccount::INIT_SPACE,
        payer = signer,
        seeds = [reserve_type.as_bytes()],
        bump
    )]
    pub vesting_account: Account<'info, VestingAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        init,
        payer = signer,
        associated_token::mint = mint,
        associated_token::authority = vesting_account,
        associated_token::token_program = token_program
    )]
    pub treasury_token_account: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct CreateReserveAccount<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK: Not written to
    pub beneficiary: SystemAccount<'info>,
    #[account(mut)]
    pub owner_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(has_one = owner)]
    pub vesting_account: Account<'info, VestingAccount>,
    #[account(
        init,
        space = 8 + ReserveAccount::INIT_SPACE,
        payer = owner,
        seeds = [b"reserve", vesting_account.key().as_ref()],
        bump
    )]
    pub reserve_account: Account<'info, ReserveAccount>,
    #[account(mut)]
    pub treasury_token_account: InterfaceAccount<'info, TokenAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(reserve_type: String)]
pub struct ClaimTokens<'info> {
    #[account(mut)]
    pub beneficiary: Signer<'info>,
    #[account(
        mut,
        seeds = [b"reserve", vesting_account.key().as_ref()],
        bump = reserve_account.bump,
        has_one = beneficiary,
        has_one = vesting_account,
    )]
    pub reserve_account: Account<'info, ReserveAccount>,
    #[account(
        mut,
        seeds = [reserve_type.as_ref()],
        bump = vesting_account.bump,
        has_one = treasury_token_account,
        has_one = mint
    )]
    pub vesting_account: Account<'info, VestingAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub treasury_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub beneficiary_token_account: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}


#[derive(Accounts)]
pub struct CloseReserveAccount<'info> {
    #[account(
        mut,
        seeds = [b"reserve", vesting_account.key().as_ref()],
        bump = reserve_account.bump,
        close = beneficiary,
        has_one = beneficiary,
        has_one = vesting_account,
    )]
    pub reserve_account: Account<'info, ReserveAccount>,
    #[account(
        seeds = [vesting_account.reserve_type.as_bytes()],
        bump = vesting_account.bump,
    )]
    pub vesting_account: Account<'info, VestingAccount>,
    #[account(mut)]
    pub beneficiary: Signer<'info>,
    pub system_program: Program<'info, System>,
}
#[account]
#[derive(InitSpace, Debug)]
pub struct VestingAccount {
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub treasury_token_account: Pubkey,
    #[max_len(50)]
    pub reserve_type: String,
    pub bump: u8,
}

#[account]
#[derive(InitSpace, Debug)]
pub struct ReserveAccount {
    pub beneficiary: Pubkey,
    pub start_time: i64,
    pub end_time: i64,
    pub total_amount: i64,
    pub amount_withdrawn: i64,
    pub cliff_time: i64,
    pub monthly_claim: i64,
    pub vesting_account: Pubkey,
    pub bump: u8,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Claiming Unavailable. Current Vesting Period Not Over Yet.")]
    ClaimNotAvailableYet,
    #[msg("Zero Balance. Vault Empty.")]
    NothingToClaim,
    #[msg("Cliff Period Not Ended Yet.")]
    CliffPeriodNotEnded,
    #[msg("CAUTION: Vesting Period Ongoing. Tokens Secured In Vault.")]
    VestingNotOver,
    #[msg("CAUTION: TOKEN LIMIT SURPASSED.")]
    FundsRemaining,
    #[msg("Invalid Timestamp Calculation.")]
    InvalidTime,
    #[msg("Unauthorized User. Access Denied.")]
    AccessDenied,
}

#[event]
pub struct TokensLocked {
    pub amount: u64,
    pub locked_until: i64,
    pub unlock_amount_per_period: u64,
    pub vesting_end_time: i64,
    pub decimals: u8,
}

#[event]
pub struct TokensClaimed {
    pub claimed_amount: u64,
    pub next_claim_timestamp: i64,
    pub decimals: u8,
}

fn months_elapsed(start: DateTime<Utc>, end: DateTime<Utc>) -> i64 {
    if end < start {
        return 0;
    }
    let years_diff = end.year() - start.year();
    let months_diff = end.month() as i32 - start.month() as i32;
    let mut total_months = (years_diff * 12) + months_diff;
    if end.day() < start.day() || (end.day() == start.day() && end.time() < start.time()) {
        total_months -= 1;
    }
    if total_months < 0 {
        0
    } else {
        total_months as i64
    }
}