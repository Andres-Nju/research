//! Defines a composable Instruction type and a memory-efficient CompiledInstruction.

use crate::{pubkey::Pubkey, short_vec, system_instruction::SystemError};
use bincode::serialize;
use serde::Serialize;
use thiserror::Error;

/// Reasons the runtime might have rejected an instruction.
#[derive(Serialize, Deserialize, Debug, Error, PartialEq, Eq, Clone)]
pub enum InstructionError {
    /// Deprecated! Use CustomError instead!
    /// The program instruction returned an error
    #[error("generic instruction error")]
    GenericError,

    /// The arguments provided to a program were invalid
    #[error("invalid program argument")]
    InvalidArgument,

    /// An instruction's data contents were invalid
    #[error("invalid instruction data")]
    InvalidInstructionData,

    /// An account's data contents was invalid
    #[error("invalid account data for instruction")]
    InvalidAccountData,

    /// An account's data was too small
    #[error("account data too small for instruction")]
    AccountDataTooSmall,

    /// An account's balance was too small to complete the instruction
    #[error("insufficient funds for instruction")]
    InsufficientFunds,

    /// The account did not have the expected program id
    #[error("incorrect program id for instruction")]
    IncorrectProgramId,

    /// A signature was required but not found
    #[error("missing required signature for instruction")]
    MissingRequiredSignature,

    /// An initialize instruction was sent to an account that has already been initialized.
    #[error("instruction requires an uninitialized account")]
    AccountAlreadyInitialized,

    /// An attempt to operate on an account that hasn't been initialized.
    #[error("instruction requires an initialized account")]
    UninitializedAccount,

    /// Program's instruction lamport balance does not equal the balance after the instruction
    #[error("sum of account balances before and after instruction do not match")]
    UnbalancedInstruction,

    /// Program modified an account's program id
    #[error("instruction modified the program id of an account")]
    ModifiedProgramId,

    /// Program spent the lamports of an account that doesn't belong to it
    #[error("instruction spent from the balance of an account it does not own")]
    ExternalAccountLamportSpend,

    /// Program modified the data of an account that doesn't belong to it
    #[error("instruction modified data of an account it does not own")]
    ExternalAccountDataModified,

    /// Read-only account's lamports modified
    #[error("instruction changed the balance of a read-only account")]
    ReadonlyLamportChange,

    /// Read-only account's data was modified
    #[error("instruction modified data of a read-only account")]
    ReadonlyDataModified,

    /// An account was referenced more than once in a single instruction
    // Deprecated, instructions can now contain duplicate accounts
    #[error("instruction contains duplicate accounts")]
    DuplicateAccountIndex,

    /// Executable bit on account changed, but shouldn't have
    #[error("instruction changed executable bit of an account")]
    ExecutableModified,

    /// Rent_epoch account changed, but shouldn't have
    #[error("instruction modified rent epoch of an account")]
    RentEpochModified,

    /// The instruction expected additional account keys
    #[error("insufficient account key count for instruction")]
    NotEnoughAccountKeys,

    /// A non-system program changed the size of the account data
    #[error("non-system instruction changed account size")]
    AccountDataSizeChanged,

    /// The instruction expected an executable account
    #[error("instruction expected an executable account")]
    AccountNotExecutable,

    /// Failed to borrow a reference to account data, already borrowed
    #[error("instruction tries to borrow reference for an account which is already borrowed")]
    AccountBorrowFailed,

    /// Account data has an outstanding reference after a program's execution
    #[error("instruction left account with an outstanding reference borrowed")]
    AccountBorrowOutstanding,

    /// The same account was multiply passed to an on-chain program's entrypoint, but the program
    /// modified them differently.  A program can only modify one instance of the account because
    /// the runtime cannot determine which changes to pick or how to merge them if both are modified
    #[error("instruction modifications of multiply-passed account differ")]
    DuplicateAccountOutOfSync,

    /// Allows on-chain programs to implement program-specific error types and see them returned
    /// by the Solana runtime. A program-specific error may be any type that is represented as
    /// or serialized to a u32 integer.
    #[error("program error: {0}")]
    CustomError(u32),

    /// The return value from the program was invalid.  Valid errors are either a defined builtin
    /// error value or a user-defined error in the lower 32 bits.
    #[error("program returned invalid error code")]
    InvalidError,

    /// Executable account's data was modified
    #[error("instruction changed executable accounts data")]
    ExecutableDataModified,

    /// Executable account's lamports modified
    #[error("instruction changed the balance of a executable account")]
    ExecutableLamportChange,

    /// Executable accounts must be rent exempt
    #[error("executable accounts must be rent exempt")]
    ExecutableAccountNotRentExempt,
}

impl InstructionError {
    pub fn new_result_with_negative_lamports() -> Self {
        SystemError::ResultWithNegativeLamports.into()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Instruction {
    /// Pubkey of the instruction processor that executes this instruction
    pub program_id: Pubkey,
    /// Metadata for what accounts should be passed to the instruction processor
    pub accounts: Vec<AccountMeta>,
    /// Opaque data passed to the instruction processor
    pub data: Vec<u8>,
}

impl Instruction {
    pub fn new<T: Serialize>(program_id: Pubkey, data: &T, accounts: Vec<AccountMeta>) -> Self {
        let data = serialize(data).unwrap();
        Self {
            program_id,
            data,
            accounts,
        }
    }
}

/// Account metadata used to define Instructions
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct AccountMeta {
    /// An account's public key
    pub pubkey: Pubkey,
    /// True if an Instruction requires a Transaction signature matching `pubkey`.
    pub is_signer: bool,
    /// True if the `pubkey` can be loaded as a read-write account.
    pub is_writable: bool,
}

impl AccountMeta {
    pub fn new(pubkey: Pubkey, is_signer: bool) -> Self {
        Self {
            pubkey,
            is_signer,
            is_writable: true,
        }
    }

    pub fn new_readonly(pubkey: Pubkey, is_signer: bool) -> Self {
        Self {
            pubkey,
            is_signer,
            is_writable: false,
        }
    }
}

/// Trait for adding a signer Pubkey to an existing data structure
pub trait WithSigner {
    /// Add a signer Pubkey
    fn with_signer(self, signer: &Pubkey) -> Self;
}

impl WithSigner for Vec<AccountMeta> {
    fn with_signer(mut self, signer: &Pubkey) -> Self {
        for meta in self.iter_mut() {
            // signer might already appear in parameters
            if &meta.pubkey == signer {
                meta.is_signer = true; // found it, we're done
                return self;
            }
        }

        // signer wasn't in metas, append it after normal parameters
        self.push(AccountMeta::new_readonly(*signer, true));
        self
    }
}

/// An instruction to execute a program
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CompiledInstruction {
    /// Index into the transaction keys array indicating the program account that executes this instruction
    pub program_id_index: u8,
    /// Ordered indices into the transaction keys array indicating which accounts to pass to the program
    #[serde(with = "short_vec")]
    pub accounts: Vec<u8>,
    /// The program input data
    #[serde(with = "short_vec")]
    pub data: Vec<u8>,
}

impl CompiledInstruction {
    pub fn new<T: Serialize>(program_ids_index: u8, data: &T, accounts: Vec<u8>) -> Self {
        let data = serialize(data).unwrap();
        Self {
            program_id_index: program_ids_index,
            data,
            accounts,
        }
    }

    pub fn program_id<'a>(&self, program_ids: &'a [Pubkey]) -> &'a Pubkey {
        &program_ids[self.program_id_index as usize]
    }

    /// Visit each unique instruction account index once
    pub fn visit_each_account(
        &self,
        work: &mut dyn FnMut(usize, usize) -> Result<(), InstructionError>,
    ) -> Result<(), InstructionError> {
        let mut unique_index = 0;
        'root: for (i, account_index) in self.accounts.iter().enumerate() {
            // Note: This is an O(n^2) algorithm,
            // but performed on a very small slice and requires no heap allocations
            for account_index_before in self.accounts[..i].iter() {
                if account_index_before == account_index {
                    continue 'root; // skip dups
                }
            }
            work(unique_index, *account_index as usize)?;
            unique_index += 1;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_account_meta_list_with_signer() {
        let account_pubkey = Pubkey::new_rand();
        let signer_pubkey = Pubkey::new_rand();

        let account_meta = AccountMeta::new(account_pubkey, false);
        let signer_account_meta = AccountMeta::new(signer_pubkey, false);

        let metas = vec![].with_signer(&signer_pubkey);
        assert_eq!(metas.len(), 1);
        assert!(metas[0].is_signer);

        let metas = vec![account_meta.clone()].with_signer(&signer_pubkey);
        assert_eq!(metas.len(), 2);
        assert!(!metas[0].is_signer);
        assert!(metas[1].is_signer);
        assert_eq!(metas[1].pubkey, signer_pubkey);

        let metas = vec![signer_account_meta.clone()].with_signer(&signer_pubkey);
        assert_eq!(metas.len(), 1);
        assert!(metas[0].is_signer);
        assert_eq!(metas[0].pubkey, signer_pubkey);

        let metas = vec![account_meta, signer_account_meta].with_signer(&signer_pubkey);
        assert_eq!(metas.len(), 2);
        assert!(!metas[0].is_signer);
        assert!(metas[1].is_signer);
        assert_eq!(metas[1].pubkey, signer_pubkey);
    }

    #[test]
    fn test_visit_each_account() {
        let do_work = |accounts: &[u8]| -> (usize, usize) {
            let mut unique_total = 0;
            let mut account_total = 0;
            let mut work = |unique_index: usize, account_index: usize| {
                unique_total += unique_index;
                account_total += account_index;
                Ok(())
            };
            let instruction = CompiledInstruction::new(0, &[0], accounts.to_vec());
            instruction.visit_each_account(&mut work).unwrap();

            (unique_total, account_total)
        };

        assert_eq!((6, 6), do_work(&[0, 1, 2, 3]));
        assert_eq!((6, 6), do_work(&[0, 1, 1, 2, 3]));
        assert_eq!((6, 6), do_work(&[0, 1, 2, 3, 3]));
        assert_eq!((6, 6), do_work(&[0, 0, 1, 1, 2, 2, 3, 3]));
        assert_eq!((0, 2), do_work(&[2, 2]));
    }
}
