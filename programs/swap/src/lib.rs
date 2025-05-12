use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_instruction;
use anchor_spl::token::{Mint, TokenAccount, Token, MintTo};
use anchor_spl::associated_token::{AssociatedToken, Create};

declare_id!("A4uMCzB2tzpaPVt1oU8sdAyhwCsrPm4u3HvnS3iFMM8o");

// =====================
// STRUCTS DE CONTEXTO
// =====================

#[derive(Accounts)]
#[instruction(rate: u64)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = payer,
        seeds = [b"program-state"],
        bump,
        space = 8 + 32 + 8 + 32,
    )]
    pub program_state: Account<'info, ProgramState>,

    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Apenas para salvar pubkey
    pub owner: UncheckedAccount<'info>,

    /// CHECK: Apenas para salvar pubkey
    pub vault: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut)]
    pub user_lbx_ata: Account<'info, TokenAccount>,

    #[account(mut)]
    pub lbx_mint: Account<'info, Mint>,

    #[account(
        seeds = [b"program-state"],
        bump,
    )]
    pub program_state: Account<'info, ProgramState>,

    /// CHECK: Garantido por program_state.vault
    #[account(mut)]
    pub vault: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(
        mut,
        seeds = [b"program-state"],
        bump,
        has_one = owner
    )]
    pub program_state: Account<'info, ProgramState>,

    pub owner: Signer<'info>,
}

// =====================
// CONTA DE ESTADO
// =====================

#[account]
pub struct ProgramState {
    pub owner: Pubkey,
    pub rate: u64,
    pub vault: Pubkey,
}

// =====================
// CÓDIGO PRINCIPAL
// =====================

#[program]
pub mod swap {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, rate: u64) -> Result<()> {
        let state = &mut ctx.accounts.program_state;
        state.owner = *ctx.accounts.owner.key;
        state.vault = *ctx.accounts.vault.key;
        state.rate = rate;
        Ok(())
    }

    pub fn swap(ctx: Context<Swap>, sol_amount: u64) -> Result<()> {
        let rate = ctx.accounts.program_state.rate;
        require!(sol_amount > 0, SwapError::NoSolSent);

        let amount_to_mint = sol_amount
            .checked_mul(rate)
            .ok_or(SwapError::MintAmountOverFlow)?;

        // Criar ATA se necessário
        let ata_ctx = CpiContext::new(
            ctx.accounts.associated_token_program.to_account_info(),
            Create {
                payer: ctx.accounts.user.to_account_info(),
                associated_token: ctx.accounts.user_lbx_ata.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
                mint: ctx.accounts.lbx_mint.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                token_program: ctx.accounts.token_program.to_account_info(),
            },
        );
        anchor_spl::associated_token::create(ata_ctx)?;

        // Mintar tokens com signer
        let bump = ctx.bumps.program_state;
        let signer_seeds: &[&[u8]] = &[b"program-state", &[bump]];
        let signer: &[&[&[u8]]] = &[signer_seeds];

        let mint_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.lbx_mint.to_account_info(),
                to: ctx.accounts.user_lbx_ata.to_account_info(),
                authority: ctx.accounts.program_state.to_account_info(),
            },
            signer,
        );
        anchor_spl::token::mint_to(mint_ctx, amount_to_mint)?;

        // Transferência segura de SOL para o vault
        let transfer_ix = system_instruction::transfer(
            &ctx.accounts.user.key(),
            &ctx.accounts.vault.key(),
            sol_amount,
        );
        anchor_lang::solana_program::program::invoke(
            &transfer_ix,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.vault.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        Ok(())
    }

    pub fn update_config(
        ctx: Context<UpdateConfig>,
        new_rate: u64,
        new_vault: Option<Pubkey>,
    ) -> Result<()> {
        let state = &mut ctx.accounts.program_state;

        require_keys_eq!(ctx.accounts.owner.key(), state.owner, SwapError::Unauthorized);

        state.rate = new_rate;
        state.vault = new_vault.unwrap_or(ctx.accounts.owner.key());

        msg!("✅ Rate atualizado para {} LBX por SOL", new_rate);
        msg!("✅ Vault atualizado para {}", state.vault);
        Ok(())
    }
}

// =====================
// CÓDIGOS DE ERRO
// =====================

#[error_code]
pub enum SwapError {
    #[msg("Nenhum SOL foi enviado.")]
    NoSolSent,

    #[msg("Erro ao calcular a quantidade de tokens a mintar.")]
    MintAmountOverFlow,

    #[msg("Somente o owner pode executar esta ação.")]
    Unauthorized,
}
