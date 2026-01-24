use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_instruction;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};
use solana_security_txt::security_txt;

security_txt! {
    name: "Presale Buy Now Contract",
    project_url: "https://saurs.ai",
    contacts: "email:reachout@saurs.ai",
    preferred_languages: "en",
    source_code: "https://github.com/xys/Vesting-Verify",
    policy: "https://saurs.ai/buynow",
    acknowledgements: "The SaurAI Labs"
}


declare_id!("SAURbWr37gXUYK9a15ppcppys1ktnmyVDFkCCyC2ePn");


pub const FEED_ID: &str = "ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d"; // SOL/USD price feed id from https://pyth.network/developers/price-feed-ids


#[program]
pub mod swap {
    use super::*;


    pub fn initialize(
        ctx: Context<Initialize>,
        index: u64,
        price_per_token: u64,
    ) -> Result<()> {
        require!(
            ctx.accounts.authority.key() == pubkey!("3c1gFBMmZFrDTgUz2HH8yhhbfqibdwfK14QtHRiQLYE1"),
            CustomError::InvalidAuth
        );
        let vault = &mut ctx.accounts.vault;
        vault.index = index;
        vault.token_mint = ctx.accounts.token_mint.key();
        vault.vault_token_account = ctx.accounts.vault_token_account.key();
        vault.price_per_token = price_per_token;
        vault.total_tokens = 0;
        vault.bump = ctx.bumps.vault;
        vault.owner = ctx.accounts.authority.key();
        Ok(())
    }


    pub fn update_price(ctx: Context<UpdatePrice>, new_price: u64) -> Result<()> {
        require!(
            ctx.accounts.authority.key() == pubkey!("3c1gFBMmZFrDTgUz2HH8yhhbfqibdwfK14QtHRiQLYE1"),
            CustomError::InvalidAuth
        );
        let vault = &mut ctx.accounts.vault;
        vault.price_per_token = new_price;
        Ok(())
    }


    pub fn deposit_tokens(ctx: Context<DepositTokens>, amount: u64) -> Result<()> {
        require!(
            ctx.accounts.authority.key() == pubkey!("3c1gFBMmZFrDTgUz2HH8yhhbfqibdwfK14QtHRiQLYE1"),
            CustomError::InvalidAuth
        );
        let cpi_accounts = TransferChecked {
            mint: ctx.accounts.token_mint.to_account_info(),
            from: ctx.accounts.admin_token_account.to_account_info(),
            to: ctx.accounts.vault_token_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        transfer_checked(cpi_ctx, amount, ctx.accounts.token_mint.decimals)?;
        let vault = &mut ctx.accounts.vault;
        vault.total_tokens += amount;
        Ok(())
    }
    pub fn withdraw_tokens(ctx: Context<WithdrawTokens>, amount: u64) -> Result<()> {
     
        require!(
            ctx.accounts.authority.key() == pubkey!("3c1gFBMmZFrDTgUz2HH8yhhbfqibdwfK14QtHRiQLYE1"),
            CustomError::InvalidAuth
        );
        let cpi_accounts = TransferChecked {
            mint: ctx.accounts.token_mint.to_account_info(),
            from: ctx.accounts.vault_token_account.to_account_info(),
            to: ctx.accounts.admin_token_account.to_account_info(),
            authority: ctx.accounts.vault_signer.to_account_info(),
        };
        let mint = ctx.accounts.token_mint.key();
        let seeds = &[
            b"vault",
            mint.as_ref(),
            &ctx.accounts.vault.index.to_le_bytes(),
            &[ctx.accounts.vault.bump],
        ];
        let signer_seeds = &[&seeds[..]];
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        transfer_checked(cpi_ctx, amount, ctx.accounts.token_mint.decimals)?;
        ctx.accounts.vault.total_tokens = ctx.accounts.vault.total_tokens.saturating_sub(amount);
        Ok(())
    }

    pub fn transfer_from_vault(ctx: Context<TransferFromVault>, amount: u64) -> Result<()> {
        require!(
            ctx.accounts.authority.key() == pubkey!("3c1gFBMmZFrDTgUz2HH8yhhbfqibdwfK14QtHRiQLYE1"),
            CustomError::InvalidAuth
        );
        let cpi_accounts = TransferChecked {
            mint: ctx.accounts.token_mint.to_account_info(),
            from: ctx.accounts.vault_token_account.to_account_info(),
            to: ctx.accounts.destination_token_account.to_account_info(),
            authority: ctx.accounts.vault_signer.to_account_info(),
        };
        let mint = ctx.accounts.token_mint.key();
        let seeds = &[
            b"vault",
            mint.as_ref(),
            &ctx.accounts.vault.index.to_le_bytes(),
            &[ctx.accounts.vault.bump],
        ];
        let signer_seeds = &[&seeds[..]];
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        transfer_checked(cpi_ctx, amount, ctx.accounts.token_mint.decimals)?;
        ctx.accounts.vault.total_tokens = ctx.accounts.vault.total_tokens.saturating_sub(amount);
        Ok(())
    }
    pub fn purchase_tokens(ctx: Context<PurchaseTokens>, amount: u64) -> Result<()> {
        let decimals = ctx.accounts.token_mint.decimals;
        let tkn_amount = amount / 10u64.pow(decimals as u32);
        require!(
            tkn_amount <= 1000000,
            CustomError::TokenLimit
        );
        let vault = &mut ctx.accounts.vault;
        let price_update = &ctx.accounts.sol_usd_price;
        let maximum_age: u64 = 600;
        let price_data = price_update.get_price_no_older_than(&Clock::get()?, maximum_age, &get_feed_id_from_hex(FEED_ID)?,)?;
        let expo: i32 = price_data.exponent;
       
        require!(
            amount <= vault.total_tokens,
            CustomError::InsufficientTokens
        );
   
       let usdt_decimals = 6u32;
      let sol_decimals = 9u32;
     
      // Use u128 for all arithmetic
      let total_price = (tkn_amount as u128)
          .checked_mul(vault.price_per_token as u128)
          .ok_or(CustomError::Overflow)?;
     
      // price as positive u128
      let price = price_data.price.abs() as u128;
     
      // Calculate denominator: price * 10^usdt_decimals
      let denominator = price
          .checked_mul(10u128.pow(usdt_decimals))
          .ok_or(CustomError::Overflow)?;
     
      // Calculate numerator: total_price * 10^sol_decimals * 10^(-expo)
      let mut numerator = total_price
          .checked_mul(10u128.pow(sol_decimals))
          .ok_or(CustomError::Overflow)?;
     
      if expo < 0 {
          numerator = numerator
              .checked_mul(10u128.pow((-expo) as u32))
              .ok_or(CustomError::Overflow)?;
      } else {
          numerator = numerator
              .checked_div(10u128.pow(expo as u32))
              .ok_or(CustomError::Overflow)?;
      }
     
      // Final division, then cast to u64
      let amount_to_pay = numerator
          .checked_div(denominator)
          .ok_or(CustomError::Overflow)? as u64;
        let ix = system_instruction::transfer(
            ctx.accounts.buyer.key,
            ctx.accounts.admin.key,
            amount_to_pay,
        );
        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                ctx.accounts.buyer.to_account_info(),
                ctx.accounts.admin.to_account_info(),
            ],
        )?;


        let cpi_accounts = TransferChecked {
            mint: ctx.accounts.token_mint.to_account_info(),
            from: ctx.accounts.vault_token_account.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.vault_signer.to_account_info(),
        };
        let mint = ctx.accounts.token_mint.key();
        let seeds = &[
            b"vault",
            mint.as_ref(),
            &vault.index.to_le_bytes(),
            &[vault.bump],
        ];
        let signer_seeds = &[&seeds[..]];
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        transfer_checked(cpi_ctx, amount, ctx.accounts.token_mint.decimals)?;
        vault.total_tokens -= amount;
        Ok(())
    }
    pub fn close_vault(ctx: Context<CloseVault>) -> Result<()> {


        let vault = &mut ctx.accounts.vault;
        // Transfer all tokens in the vault to the admin's account
        let vault_token_account = &ctx.accounts.vault_token_account;
        let admin_token_account = &ctx.accounts.admin_token_account;
        let token_mint = &ctx.accounts.token_mint;
        let token_program = &ctx.accounts.token_program;
        let vault_signer = &ctx.accounts.vault_signer;


        let amount = vault_token_account.amount;


        if amount > 0 {
            let cpi_accounts = TransferChecked {
                mint: token_mint.to_account_info(),
                from: vault_token_account.to_account_info(),
                to: admin_token_account.to_account_info(),
                authority: vault_signer.to_account_info(),
            };
            let tk_mint = token_mint.key();
            let seeds = &[
                b"vault",
                tk_mint.as_ref(),
                &vault.index.to_le_bytes(),
                &[vault.bump],
            ];
            let signer_seeds = &[&seeds[..]];
            let cpi_program = token_program.to_account_info();
            let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
            transfer_checked(cpi_ctx, amount, token_mint.decimals)?;
        }
        Ok(())
    }
}


#[derive(Accounts)]
#[instruction(index: u64)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        seeds = [b"vault", token_mint.key().as_ref(), index.to_le_bytes().as_ref()],
        bump,
        payer = authority,
        space = 8 + Vault::INIT_SPACE
    )]
    pub vault: Account<'info, Vault>,
    pub token_mint: InterfaceAccount<'info, Mint>,
    #[account(
        init,
        payer = authority,
        associated_token::mint = token_mint,
        associated_token::authority = vault,
        associated_token::token_program = token_program
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}


#[derive(Accounts)]
pub struct UpdatePrice<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub vault: Account<'info, Vault>,
}


#[derive(Accounts)]
pub struct DepositTokens<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub vault: Account<'info, Vault>,
    pub token_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub admin_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
}


#[derive(Accounts)]
pub struct WithdrawTokens<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub vault: Account<'info, Vault>,
    pub token_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub admin_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: PDA signer for vault
    #[account(
        seeds = [b"vault", token_mint.key().as_ref(), vault.index.to_le_bytes().as_ref()],
        bump = vault.bump
    )]
    pub vault_signer: UncheckedAccount<'info>,
    pub token_program: Interface<'info, TokenInterface>,
}




#[derive(Accounts)]
pub struct PurchaseTokens<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    /// CHECK: This is the admin recipient (must match ADMIN)
    #[account(mut, address = pubkey!("3c1gFBMmZFrDTgUz2HH8yhhbfqibdwfK14QtHRiQLYE1"))]
    pub admin: UncheckedAccount<'info>,
    #[account(mut)]
    pub vault: Account<'info, Vault>,
    pub token_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: PDA signer for vault
    #[account(
        seeds = [b"vault", token_mint.key().as_ref(), vault.index.to_le_bytes().as_ref()],
        bump = vault.bump
    )]
    pub vault_signer: UncheckedAccount<'info>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,


    // Add this line:
    // Add this line:
    /// CHECK: Pyth price account
    #[account()]
    pub sol_usd_price: Account<'info, PriceUpdateV2>,
}


#[derive(Accounts)]
pub struct CloseVault<'info> {
    #[account(mut, close = owner)]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut)]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub admin_token_account: InterfaceAccount<'info, TokenAccount>,
    pub token_mint: InterfaceAccount<'info, Mint>,
    /// CHECK: PDA signer for vault
    #[account(
        seeds = [b"vault", token_mint.key().as_ref(), vault.index.to_le_bytes().as_ref()],
        bump = vault.bump
    )]
    pub vault_signer: UncheckedAccount<'info>,
    pub token_program: Interface<'info, TokenInterface>,
}

#[derive(Accounts)]
pub struct TransferFromVault<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub destination_token_account: InterfaceAccount<'info, TokenAccount>,
    pub token_mint: InterfaceAccount<'info, Mint>,
    /// CHECK: PDA signer for vault
    #[account(
        seeds = [b"vault", token_mint.key().as_ref(), vault.index.to_le_bytes().as_ref()],
        bump = vault.bump
    )]
    pub vault_signer: UncheckedAccount<'info>,
    pub token_program: Interface<'info, TokenInterface>,
}


#[account]
#[derive(InitSpace)]
pub struct Vault {
    pub index: u64,
    pub token_mint: Pubkey,
    pub vault_token_account: Pubkey,
    pub price_per_token: u64,
    pub total_tokens: u64,
    pub bump: u8,
    pub owner: Pubkey,
}


#[error_code]
pub enum CustomError {
    #[msg("Unauthorized User. Access Denied.")]
    InvalidAuth,
    #[msg("Presale Phase Completed. Please Wait For The Next Phase.")]
    InsufficientTokens,
    #[msg("Arithmetic Overflow")]
    Overflow,
    #[msg("Vault not expired. Please Withdraw All Tokens First.")]
    VaultNotExpired,
    #[msg("You can only purchase SAURAI equivalent upto 1000 USD per transaction")]
    TokenLimit,
}