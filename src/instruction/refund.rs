use std::alloc::realloc;

use pinocchio::{ProgramResult, account_info::AccountInfo, instruction::{Seed, Signer}, program_error::ProgramError, pubkey::find_program_address};
use pinocchio_log::log;
use pinocchio_token::{instructions::{CloseAccount, Transfer}, state::TokenAccount};

use crate::{AssociatedTokenAccount, Escrow};

pub struct Refund<'a> {
    pub accounts : RefundAccounts<'a>
}

impl<'a> TryFrom<&'a [AccountInfo]> for Refund<'a> {
    type Error = ProgramError;
    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let accounts = RefundAccounts::try_from(accounts)?;


        AssociatedTokenAccount::init_if_needed(
            accounts.maker_ata_a,
            accounts.mint_a, 
            accounts.maker, 
            accounts.maker, 
            accounts.system_program, 
            accounts.token_program
        )?;

        Ok(Self { accounts })
    }
    
}


impl<'a> Refund<'a>  {
    pub const DISCRIMINATOR: &'a u8 = &2;

    pub fn process(&mut self) -> ProgramResult {
        let escrow_data = self.accounts.escrow.try_borrow_data()?;
        let escrow = Escrow::load(&escrow_data)?;


        let seed_binding = escrow.seed.to_le_bytes();
        let bump_binding = escrow.bump;


        let seeds = &[
            Seed::from(b"escrow"),
            Seed::from(self.accounts.maker.key().as_ref()),
            Seed::from(&seed_binding),
            Seed::from(&bump_binding)
        ];

        let signer = [Signer::from(seeds)];


        let amount: u64;

        {
            let vault = TokenAccount::from_account_info(self.accounts.vault)?;
            amount = vault.amount();
        }

        Transfer {
            from: self.accounts.vault,
            to: self.accounts.maker_ata_a,
            authority: self.accounts.escrow,
            amount
        }.invoke_signed(&signer)?;


        CloseAccount {
            account: self.accounts.vault,
            destination: self.accounts.maker,
            authority: self.accounts.escrow
        }.invoke_signed(&signer)?;


        {
            drop(escrow_data);
            let mut data: pinocchio::account_info::RefMut<'_, [u8]> = self.accounts.escrow.try_borrow_mut_data()?;
            data[0] = 0xff;
        }


        *self.accounts.maker.try_borrow_mut_lamports()? += *self.accounts.escrow.try_borrow_lamports()?;
        self.accounts.escrow.realloc(1, true)?;
        self.accounts.escrow.close()?;
        



        
        Ok(())
    }
    
}

pub struct RefundAccounts<'a> {
    pub maker: &'a AccountInfo,
    pub escrow: &'a AccountInfo,
    pub mint_a: &'a AccountInfo,
    pub vault: &'a AccountInfo,
    pub maker_ata_a: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
    pub token_program: &'a AccountInfo,

}

impl <'a> TryFrom<&'a [AccountInfo]> for RefundAccounts<'a> {
    type Error = ProgramError;
    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [maker, escrow, mint_a, vault, maker_ata_a, system_program, token_program, _] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };


        if !maker.is_signer() {
            return Err(ProgramError::MissingRequiredSignature)
        }

        if !maker.is_owned_by(&pinocchio_system::ID) {
            return Err(ProgramError::InvalidAccountOwner);
        }


        if !escrow.is_owned_by(&crate::ID){
            return Err(ProgramError::InvalidAccountOwner);
        }

        let escrow_data: &Escrow;
        let data;
        {
            data = escrow.try_borrow_data()?;
            escrow_data = Escrow::load(&data)?;

        }

        let (address, _) = find_program_address(
           &[b"escrow", maker.key().as_ref(), &escrow_data.seed.to_le_bytes()],
            &crate::ID);

        if escrow.key() != &address {
            return Err(ProgramError::InvalidAccountData);
        } 

        if mint_a.data_len() != pinocchio_token::state::Mint::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        
        if  !mint_a.is_owned_by(&pinocchio_token::ID) {
            return Err(ProgramError::InvalidAccountOwner)
        }
        

        if vault.data_len() != pinocchio_token::state::TokenAccount::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        if !vault.is_owned_by(&pinocchio_token::ID) {
            return Err(ProgramError::InvalidAccountOwner)
        }


        {
            
            log!("maker ata owner: {} ", maker_ata_a.owner());
        }
                
        // log!("here 4");
        // if !maker_ata_a.is_owned_by(&pinocchio_token::ID) {
        //     return Err(ProgramError::InvalidAccountOwner)
        // }
        // log!("maker ata length: {} token account length {} " , maker_ata_a.data_len(), pinocchio_token::state::TokenAccount::LEN);
        // if maker_ata_a.data_len() != pinocchio_token::state::TokenAccount::LEN {
        //     return Err(ProgramError::InvalidAccountData);
        // }

        if system_program.key() != &pinocchio_system::ID {
            return Err(ProgramError::InvalidAccountData);
        }

        if token_program.key() != &pinocchio_token::ID {
            return Err(ProgramError::InvalidAccountData);
        }


        Ok(Self { maker, escrow, mint_a, vault, maker_ata_a, system_program, token_program })
    }
}