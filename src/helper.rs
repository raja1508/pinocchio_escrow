use pinocchio::{ProgramResult, account_info::AccountInfo, program_error::ProgramError, pubkey::find_program_address};
use pinocchio_associated_token_account::instructions::Create;


pub struct AssociatedTokenAccount;

impl AssociatedTokenAccount {
    pub fn check(
        account: &AccountInfo,
        authority: &AccountInfo,
        mint: &AccountInfo,
        token_program: &AccountInfo,
    ) -> Result<(), ProgramError> {
        
        if !account.is_owned_by(&pinocchio_token::ID) {
            return Err(ProgramError::InvalidAccountOwner);
        }

        if account.data_len().ne(&pinocchio_token::state::TokenAccount::LEN) {
            return Err(ProgramError::InvalidAccountData);
        }

        if find_program_address(
            &[authority.key(), token_program.key(), mint.key()],
            &pinocchio_associated_token_account::ID,
        ).0.ne(account.key()) {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(())
    }

    pub fn init(
        account: &AccountInfo, 
        mint: &AccountInfo, 
        payer: &AccountInfo, 
        owner: &AccountInfo, 
        system_program: &AccountInfo, 
        token_program: &AccountInfo
    ) -> ProgramResult {
        Create{
            funding_account: payer,
            account,
            wallet: owner,
            mint,
            system_program,
            token_program,
        }.invoke()
    }

    pub fn init_if_needed(
        account: &AccountInfo, 
        mint: &AccountInfo, 
        payer: &AccountInfo, 
        owner: &AccountInfo, 
        system_program: &AccountInfo, 
        token_program: &AccountInfo
    ) -> ProgramResult {
        match Self::check(account, payer, mint, token_program) {
            Ok(_) => Ok(()),
            Err(_) => Self::init(account, mint, payer, owner, system_program, token_program),
        }
    }
}