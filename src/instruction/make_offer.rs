use pinocchio::{ProgramResult, account_info::AccountInfo, instruction::{Seed, Signer}, program_error::ProgramError, pubkey::find_program_address, sysvars::{Sysvar, rent::Rent}};
use pinocchio_associated_token_account::instructions::Create;
use pinocchio_log::log;
use pinocchio_system::instructions::{CreateAccount};
use pinocchio_token::instructions::Transfer;

use crate::Escrow;


pub struct Make<'a> {
    pub accounts: MakeAccount<'a>,
    pub instruction_data: MakeInstructionData,
    pub bump: u8
}


impl <'a> TryFrom<(&'a [u8], &'a [AccountInfo])> for Make<'a>{
    type Error = ProgramError;
    fn try_from((data, accounts): (&'a [u8], &'a [AccountInfo])) -> Result<Self, Self::Error> {
        let accounts = MakeAccount::try_from(accounts)?;
        let instruction_data = MakeInstructionData::try_from(data)?;

        let (_, bump) = find_program_address(
            &[b"escrow", accounts.maker.key(), &instruction_data.seed.to_le_bytes()], 
            &crate::ID
        );

        let seed_binding = instruction_data.seed.to_le_bytes();
        let bump_binding = [bump];
        let escrow_seeds = [
            Seed::from(b"escrow"),
            Seed::from(accounts.maker.key().as_ref()),
            Seed::from(&seed_binding),
            Seed::from(&bump_binding)
        ];

        let signer = [Signer::from(&escrow_seeds)];

        CreateAccount{
            from: accounts.maker,
            to: accounts.escrow,
            lamports: Rent::get()?.minimum_balance(Escrow::LEN) as u64,
            space: Escrow::LEN as u64,
            owner: &crate::ID
        }.invoke_signed(&signer)?;

        Create {
            funding_account: accounts.maker,  // paying account
            account:  accounts.vault,         // account to be created as ata
            wallet: accounts.escrow,            // account which will have authority over the creating ata 
            mint: accounts.mint_a,
            system_program: accounts.system_program,
            token_program: accounts.token_program,

        }.invoke()?;

        Ok(Self { accounts, instruction_data, bump})
    }
    
}




impl <'a> Make<'a> {
    pub const DISCRIMINATOR: &'a u8 = &0;

    pub fn process(&mut self) -> ProgramResult {
        let mut data = self.accounts.escrow.try_borrow_mut_data()?;
        let escrow = Escrow::load_mut(data.as_mut())?;

        escrow.set_inner(
            self.instruction_data.seed, 
            *self.accounts.maker.key(), 
            *self.accounts.mint_a.key(), 
            *self.accounts.mint_b.key(), 
            self.instruction_data.receive,
            [self.bump]
            
        );

        Transfer {
            from: self.accounts.maker_ata_a,
            to: self.accounts.vault,
            authority: self.accounts.maker,
            amount: self.instruction_data.amount
        }.invoke()?;
        
        Ok(())
    }
}
pub struct MakeAccount<'a> {
    pub maker: &'a AccountInfo,
    pub escrow: &'a AccountInfo,
    pub mint_a: &'a AccountInfo,
    pub mint_b: &'a AccountInfo,
    pub maker_ata_a: &'a AccountInfo,
    pub vault: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
    pub token_program: &'a AccountInfo
}

impl<'a> TryFrom<&'a [AccountInfo]> for MakeAccount<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [maker, escrow, mint_a, mint_b, maker_ata_a, vault, system_program, token_program, _] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !maker.is_signer() {
            return Err(ProgramError::InvalidAccountOwner);
        }

        
        log!("Instruction data length: {}", mint_a.data_len());
        log!("Mint length: {}", pinocchio_token::state::Mint::LEN );

        if mint_a.data_len() != pinocchio_token::state::Mint::LEN {
            
            return Err(ProgramError::InvalidAccountData);
        }

        if  !mint_a.is_owned_by(&pinocchio_token::ID) {
            return Err(ProgramError::InvalidAccountOwner)
        }

        if mint_b.data_len() != pinocchio_token::state::Mint::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        if  !mint_b.is_owned_by(&pinocchio_token::ID) {
            return Err(ProgramError::InvalidAccountOwner)
        }


        

        Ok(Self { maker, escrow, mint_a, mint_b, maker_ata_a, vault, system_program, token_program })
    }
}


pub struct MakeInstructionData {
    pub seed: u64,
    pub receive: u64,
    pub amount: u64

}

impl <'a> TryFrom<&'a [u8]> for MakeInstructionData {
    type Error = ProgramError;
    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len() != size_of::<u64>() * 3{
            log!("error comes from here 1 ");
            return Err(ProgramError::InvalidInstructionData);
        }

        let seed = u64::from_le_bytes(data[0..8].try_into().unwrap());
        let receive = u64::from_le_bytes(data[8..16].try_into().unwrap());
        let amount = u64::from_le_bytes(data[16..24].try_into().unwrap());

        log!("seed {} receive {} amount {} ", seed, receive, amount);

        if amount == 0 { 
            log!("error comes from here 2");
            return Err(ProgramError::InvalidInstructionData);
        }


        Ok(Self { seed, receive, amount })
    }
}

