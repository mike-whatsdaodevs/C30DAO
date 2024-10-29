use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::metadata::mpl_token_metadata::types::DataV2;
use anchor_spl::metadata::{
    create_metadata_accounts_v3, CreateMetadataAccountsV3, MetadataAccount,
};

use anchor_spl::token::{self, Mint, Token, TokenAccount};
declare_id!("6WvQ9rhpzkyxY62gqApZLmAtd52XRdo3McEJT5jEQ1Br");

#[program]
pub mod dao_voting {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        metadata: TokenMetadata,
        st_metadata: TokenMetadata,
    ) -> Result<()> {
        let global_state = &mut ctx.accounts.global_state;
        global_state.admin = ctx.accounts.admin.key();
        global_state.governance_token_mint = ctx.accounts.governance_token_mint.key();
        global_state.st_governance_token_mint = ctx.accounts.st_governance_token_mint.key();

        let governance_token_mint_seeds = &[
            b"governance_token_mint".as_ref(),
            &[ctx.bumps.governance_token_mint],
        ];
        let signer = &[&governance_token_mint_seeds[..]];

        let data = DataV2 {
            name: metadata.name,
            symbol: metadata.symbol,
            uri: metadata.uri,
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        };

        let cpi_context = CpiContext::new_with_signer(
            ctx.accounts.token_metadata_program.to_account_info(),
            CreateMetadataAccountsV3 {
                metadata: ctx.accounts.metadata.to_account_info(),
                mint: ctx.accounts.governance_token_mint.to_account_info(),
                mint_authority: ctx.accounts.admin.to_account_info(),
                payer: ctx.accounts.admin.to_account_info(),
                update_authority: ctx.accounts.admin.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
            },
            signer,
        );

        create_metadata_accounts_v3(cpi_context, data, true, true, None)?;
        let metadata_account =
            MetadataAccount::try_deserialize(&mut &ctx.accounts.metadata.data.borrow()[..])?;
        msg!("Governance token mint created successfully with metadata.");
        msg!("Metadata name: {}", metadata_account.name);
        msg!("Metadata symbol: {}", metadata_account.symbol);
        msg!("Metadata URI: {}", metadata_account.uri);

        // Create metadata for st_governance token
        let global_state_seeds = &[b"global_state".as_ref(), &[ctx.bumps.global_state]];
        let global_state_signer = &[&global_state_seeds[..]];

        let st_data = DataV2 {
            name: st_metadata.name,
            symbol: st_metadata.symbol,
            uri: st_metadata.uri,
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        };

        let st_cpi_context = CpiContext::new_with_signer(
            ctx.accounts.token_metadata_program.to_account_info(),
            CreateMetadataAccountsV3 {
                metadata: ctx.accounts.st_metadata.to_account_info(),
                mint: ctx.accounts.st_governance_token_mint.to_account_info(),
                mint_authority: ctx.accounts.global_state.to_account_info(),
                payer: ctx.accounts.admin.to_account_info(),
                update_authority: ctx.accounts.admin.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
            },
            global_state_signer,
        );

        create_metadata_accounts_v3(st_cpi_context, st_data, true, true, None)?;

        Ok(())
    }

    pub fn convert_to_st_governance(
        ctx: Context<ConvertToStGovernance>,
        amount: u64,
    ) -> Result<()> {
        // Burn governance tokens
        token::burn(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Burn {
                    mint: ctx.accounts.governance_token_mint.to_account_info(),
                    from: ctx.accounts.user_governance_token_account.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount,
        )?;

        // Mint st_governance tokens
        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::MintTo {
                    mint: ctx.accounts.st_governance_token_mint.to_account_info(),
                    to: ctx
                        .accounts
                        .user_st_governance_token_account
                        .to_account_info(),
                    authority: ctx.accounts.global_state.to_account_info(),
                },
                &[&[b"global_state".as_ref(), &[ctx.bumps.global_state]]],
            ),
            amount,
        )?;

        Ok(())
    }

    pub fn create_st_a_and_vault(
        ctx: Context<CreateStAAndVault>,
        vault_id: u64,
        max_vote_cap: u128,
        deadline: i64,
        metadata: TokenMetadata,
    ) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.owner = ctx.accounts.admin.key();
        vault.governance_token_mint = ctx.accounts.global_state.governance_token_mint;
        vault.st_governance_token_mint = ctx.accounts.global_state.st_governance_token_mint;
        vault.vote_token_mint = ctx.accounts.vote_token_mint.key();
        vault.project_token_mint = Pubkey::default(); // Initialize with default value
        vault.convert_time = 0; // Initialize with default value
        vault.total_burned = 0;
        vault.total_deposited = 0;
        vault.vault_id = vault_id;
        vault.max_vote_cap = max_vote_cap;
        vault.deadline = deadline;

        let binding = vault_id.to_le_bytes();
        let vault_seeds = &[b"vault", binding.as_ref(), &[ctx.bumps.vault]];
        let vault_signer = &[&vault_seeds[..]];

        let data = DataV2 {
            name: metadata.name,
            symbol: metadata.symbol,
            uri: metadata.uri,
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        };

        let cpi_context = CpiContext::new_with_signer(
            ctx.accounts.token_metadata_program.to_account_info(),
            CreateMetadataAccountsV3 {
                metadata: ctx.accounts.metadata.to_account_info(),
                mint: ctx.accounts.vote_token_mint.to_account_info(),
                mint_authority: ctx.accounts.vault.to_account_info(),
                payer: ctx.accounts.admin.to_account_info(),
                update_authority: ctx.accounts.admin.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
            },
            vault_signer,
        );

        create_metadata_accounts_v3(cpi_context, data, true, true, None)?;
        let metadata_account =
            MetadataAccount::try_deserialize(&mut &ctx.accounts.metadata.data.borrow()[..])?;
        msg!("Governance token mint created successfully with metadata.");
        msg!("Metadata name: {}", metadata_account.name);
        msg!("Metadata symbol: {}", metadata_account.symbol);
        msg!("Metadata URI: {}", metadata_account.uri);

        Ok(())
    }

    pub fn vote(ctx: Context<Vote>, vault_id: u64, amount: u64) -> Result<()> {
        let vault = &mut ctx.accounts.vault;

        // Check if voting is still allowed
        require!(
            Clock::get()?.unix_timestamp < vault.deadline,
            ErrorCode::VotingEnded
        );

        // Check if the new total burned amount exceeds the max vote cap
        let new_total_burned = vault
            .total_burned
            .checked_add(amount as u128)
            .ok_or(ErrorCode::VoteOverflow)?;
        require!(
            new_total_burned <= vault.max_vote_cap,
            ErrorCode::MaxVoteCapExceeded
        );

        // Burn st_governance tokens instead of governance tokens
        token::burn(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Burn {
                    mint: ctx.accounts.st_governance_token_mint.to_account_info(),
                    from: ctx
                        .accounts
                        .user_st_governance_token_account
                        .to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount,
        )?;

        // Mint vote token
        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::MintTo {
                    mint: ctx.accounts.vote_token_mint.to_account_info(),
                    to: ctx.accounts.user_vote_token_account.to_account_info(),
                    authority: vault.to_account_info(),
                },
                &[&[
                    b"vault",
                    &vault_id.to_le_bytes().as_ref(),
                    &[ctx.bumps.vault],
                ]],
            ),
            amount,
        )?;

        // Update user vault
        let user_vault = &mut ctx.accounts.user_vault;
        user_vault.burned_amount = user_vault
            .burned_amount
            .checked_add(amount as u128)
            .ok_or(ErrorCode::VoteOverflow)?;
        user_vault.user = ctx.accounts.user.key();
        user_vault.vault = vault.key();

        // Update total votes in vault
        vault.total_burned = new_total_burned;

        Ok(())
    }

    pub fn set_project_token(
        ctx: Context<SetProjectToken>,
        project_token_mint: Pubkey,
        convert_time: i64,
    ) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.project_token_mint = project_token_mint;
        vault.convert_time = convert_time;
        Ok(())
    }

    pub fn deposit_project_tokens(ctx: Context<DepositProjectTokens>, amount: u64) -> Result<()> {
        let vault = &mut ctx.accounts.vault;

        require!(
            Clock::get()?.unix_timestamp < vault.convert_time,
            ErrorCode::DepositNotAllowed
        );

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.project_token_account.to_account_info(),
                    to: ctx.accounts.vault_token_account.to_account_info(),
                    authority: ctx.accounts.project_authority.to_account_info(),
                },
            ),
            amount,
        )?;

        vault.total_deposited = vault
            .total_deposited
            .checked_add(amount as u128)
            .ok_or(ErrorCode::DepositOverflow)?;

        Ok(())
    }

    pub fn claim_project_tokens(ctx: Context<ClaimProjectTokens>, vault_id: u64) -> Result<()> {
        let vault = &ctx.accounts.vault;
        let user_vault = &ctx.accounts.user_vault;

        require!(
            Clock::get()?.unix_timestamp >= vault.convert_time,
            ErrorCode::ClaimNotAvailable
        );

        let user_ratio = (user_vault.burned_amount as f64) / (vault.total_burned as f64);
        let claim_amount =
            ((vault.total_deposited as f64 * user_ratio) as u128).min(u64::MAX as u128) as u64;

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.vault_token_account.to_account_info(),
                    to: ctx.accounts.user_project_token_account.to_account_info(),
                    authority: ctx.accounts.vault.to_account_info(),
                },
                &[&[
                    b"vault",
                    &vault_id.to_le_bytes().as_ref(),
                    &[ctx.bumps.vault],
                ]],
            ),
            claim_amount,
        )?;

        let burn_amount = user_vault.burned_amount.min(u64::MAX as u128) as u64;

        token::burn(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Burn {
                    mint: ctx.accounts.vote_token_mint.to_account_info(),
                    from: ctx.accounts.user_vote_token_account.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            burn_amount,
        )?;

        let user_vault = &mut ctx.accounts.user_vault;
        user_vault.burned_amount = 0;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + 32 + 32 + 32,
        seeds = [b"global_state"],
        bump
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        init,
        seeds = [b"governance_token_mint"],
        bump,
        payer = admin,
        mint::decimals = 6,
        mint::authority = admin,
    )]
    pub governance_token_mint: Account<'info, Mint>,
    #[account(
        init,
        seeds = [b"st_governance_token_mint"],
        bump,
        payer = admin,
        mint::decimals = 6,
        mint::authority = global_state,
    )]
    pub st_governance_token_mint: Account<'info, Mint>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
    /// CHECK: New Metaplex Account being created
    #[account(mut)]
    pub metadata: UncheckedAccount<'info>,
    /// CHECK: Metaplex program ID
    pub token_metadata_program: UncheckedAccount<'info>,
    /// CHECK: New Metaplex Account being created for st_governance token
    #[account(mut)]
    pub st_metadata: UncheckedAccount<'info>,
}

#[derive(Accounts)]
#[instruction(vault_id: u64)]
pub struct CreateStAAndVault<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + 32 + 32 + 32 + 32 + 32 + 8 + 16 + 16 + 8 + 16 + 8,
        seeds = [b"vault", vault_id.to_le_bytes().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        seeds = [b"global_state"],
        bump,
        has_one = admin
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        init,
        seeds = [b"vote_token_mint", vault.key().as_ref()],
        bump,
        payer = admin,
        mint::decimals = 6,
        mint::authority = vault,
    )]
    pub vote_token_mint: Account<'info, Mint>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,

    #[account(mut)]
    pub admin: Signer<'info>,

    /// CHECK: New Metaplex Account being created
    #[account(mut)]
    pub metadata: UncheckedAccount<'info>,
    /// CHECK: Metaplex program ID
    pub token_metadata_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
#[instruction(vault_id: u64)]
pub struct Vote<'info> {
    #[account(mut)]
    pub vote_token_mint: Account<'info, Mint>,
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = vote_token_mint,
        associated_token::authority = user
    )]
    pub user_vote_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"vault", vault_id.to_le_bytes().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,
    #[account(
        init_if_needed,
        payer = user,
        space = 8 + 32 + 32 + 16,
        seeds = [b"user_vault", vault.key().as_ref(), user.key().as_ref()],
        bump
    )]
    pub user_vault: Account<'info, UserVault>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    #[account(mut)]
    pub st_governance_token_mint: Account<'info, Mint>,
    #[account(mut)]
    pub user_st_governance_token_account: Account<'info, TokenAccount>,
}

#[derive(Accounts)]
pub struct SetProjectToken<'info> {
    #[account(mut, has_one = owner)]
    pub vault: Account<'info, Vault>,
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct DepositProjectTokens<'info> {
    #[account(mut)]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub project_token_account: Account<'info, TokenAccount>,
    #[account(
        init_if_needed,
        payer = project_authority,
        associated_token::mint = project_token_mint,
        associated_token::authority = vault
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    pub project_token_mint: Account<'info, Mint>,
    #[account(mut)]
    pub project_authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(vault_id: u64)]
pub struct ClaimProjectTokens<'info> {
    #[account(
        mut,
        seeds = [b"vault", vault_id.to_le_bytes().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub vote_token_mint: Account<'info, Mint>,
    #[account(mut)]
    pub project_token_mint: Account<'info, Mint>,
    #[account(mut)]
    pub user_vote_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = project_token_mint,
        associated_token::authority = user
    )]
    pub user_project_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        has_one = user,
        seeds = [b"user_vault", vault.key().as_ref(), user.key().as_ref()],
        bump
    )]
    pub user_vault: Account<'info, UserVault>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct ConvertToStGovernance<'info> {
    #[account(mut)]
    pub governance_token_mint: Account<'info, Mint>,
    #[account(mut)]
    pub st_governance_token_mint: Account<'info, Mint>,
    #[account(mut)]
    pub user_governance_token_account: Account<'info, TokenAccount>,
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = st_governance_token_mint,
        associated_token::authority = user
    )]
    pub user_st_governance_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(seeds = [b"global_state"], bump)]
    pub global_state: Account<'info, GlobalState>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

#[account]
pub struct GlobalState {
    pub admin: Pubkey,
    pub governance_token_mint: Pubkey,
    pub st_governance_token_mint: Pubkey,
}

#[account]
pub struct Vault {
    pub owner: Pubkey,
    pub governance_token_mint: Pubkey,
    pub vote_token_mint: Pubkey,
    pub project_token_mint: Pubkey,
    pub convert_time: i64,
    pub total_burned: u128,
    pub total_deposited: u128,
    pub vault_id: u64,
    pub max_vote_cap: u128,
    pub deadline: i64,
    pub st_governance_token_mint: Pubkey,
}

#[account]
pub struct UserVault {
    pub user: Pubkey,
    pub vault: Pubkey,
    pub burned_amount: u128,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Claim is not available yet")]
    ClaimNotAvailable,
    #[msg("Vote overflow")]
    VoteOverflow,
    #[msg("Deposit is not allowed after convert time")]
    DepositNotAllowed,
    #[msg("Deposit overflow")]
    DepositOverflow,
    #[msg("Voting period has ended")]
    VotingEnded,
    #[msg("Max vote cap exceeded")]
    MaxVoteCapExceeded,
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct TokenMetadata {
    pub name: String,
    pub symbol: String,
    pub uri: String,
}
