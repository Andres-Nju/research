//! budget contract
use bank::Account;
use bincode::{self, deserialize, serialize_into, serialized_size};
use chrono::prelude::{DateTime, Utc};
use instruction::{Instruction, Plan};
use payment_plan::{PaymentPlan, Witness};
use signature::Pubkey;
use std::io;
use transaction::Transaction;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum BudgetError {
    InsufficientFunds(Pubkey),
    ContractAlreadyExists(Pubkey),
    ContractNotPending(Pubkey),
    SourceIsPendingContract(Pubkey),
    UninitializedContract(Pubkey),
    NegativeTokens,
    DestinationMissing(Pubkey),
    FailedWitness,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct BudgetContract {
    pub initialized: bool,
    pub pending_plan: Option<Plan>,
    pub last_error: Option<BudgetError>,
}

pub const BUDGET_CONTRACT_ID: [u8; 32] = [
    1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];
impl BudgetContract {
    fn is_pending(&self) -> bool {
        self.pending_plan != None
    }
    pub fn id() -> Pubkey {
        Pubkey::new(&BUDGET_CONTRACT_ID)
    }
    pub fn check_id(contract_id: &Pubkey) -> bool {
        contract_id.as_ref() == BUDGET_CONTRACT_ID
    }

    /// Process a Witness Signature. Any payment plans waiting on this signature
    /// will progress one step.
    fn apply_signature(
        &mut self,
        keys: &[Pubkey],
        account: &mut [Account],
    ) -> Result<(), BudgetError> {
        let mut final_payment = None;
        if let Some(ref mut plan) = self.pending_plan {
            plan.apply_witness(&Witness::Signature, &keys[0]);
            final_payment = plan.final_payment();
        }

        if let Some(payment) = final_payment {
            if keys.len() < 2 || payment.to != keys[2] {
                trace!("destination missing");
                return Err(BudgetError::DestinationMissing(payment.to));
            }
            self.pending_plan = None;
            account[1].tokens -= payment.tokens;
            account[2].tokens += payment.tokens;
        }
        Ok(())
    }

    /// Process a Witness Timestamp. Any payment plans waiting on this timestamp
    /// will progress one step.
    fn apply_timestamp(
        &mut self,
        keys: &[Pubkey],
        accounts: &mut [Account],
        dt: DateTime<Utc>,
    ) -> Result<(), BudgetError> {
        // Check to see if any timelocked transactions can be completed.
        let mut final_payment = None;

        if let Some(ref mut plan) = self.pending_plan {
            plan.apply_witness(&Witness::Timestamp(dt), &keys[0]);
            final_payment = plan.final_payment();
        }

        if let Some(payment) = final_payment {
            if keys.len() < 2 || payment.to != keys[2] {
                trace!("destination missing");
                return Err(BudgetError::DestinationMissing(payment.to));
            }
            self.pending_plan = None;
            accounts[1].tokens -= payment.tokens;
            accounts[2].tokens += payment.tokens;
        }
        Ok(())
    }

    /// Deduct tokens from the source account if it has sufficient funds and the contract isn't
    /// pending
    fn apply_debits_to_budget_state(
        tx: &Transaction,
        accounts: &mut [Account],
        instruction: &Instruction,
    ) -> Result<(), BudgetError> {
        {
            // if the source account userdata is not empty, this is a pending contract
            if !accounts[0].userdata.is_empty() {
                trace!("source is pending");
                return Err(BudgetError::SourceIsPendingContract(tx.keys[0]));
            }
            if let Instruction::NewContract(contract) = &instruction {
                if contract.tokens < 0 {
                    trace!("negative tokens");
                    return Err(BudgetError::NegativeTokens);
                }

                if accounts[0].tokens < contract.tokens {
                    trace!("insufficient funds");
                    return Err(BudgetError::InsufficientFunds(tx.keys[0]));
                } else {
                    accounts[0].tokens -= contract.tokens;
                }
            };
        }
        Ok(())
    }

    /// Apply only a transaction's credits.
    /// Note: It is safe to apply credits from multiple transactions in parallel.
    fn apply_credits_to_budget_state(
        tx: &Transaction,
        accounts: &mut [Account],
        instruction: &Instruction,
    ) -> Result<(), BudgetError> {
        match instruction {
            Instruction::NewContract(contract) => {
                let plan = contract.plan.clone();
                if let Some(payment) = plan.final_payment() {
                    accounts[1].tokens += payment.tokens;
                    Ok(())
                } else {
                    let existing = Self::deserialize(&accounts[1].userdata).ok();
                    if Some(true) == existing.map(|x| x.initialized) {
                        trace!("contract already exists");
                        Err(BudgetError::ContractAlreadyExists(tx.keys[1]))
                    } else {
                        let mut state = BudgetContract::default();
                        state.pending_plan = Some(plan);
                        accounts[1].tokens += contract.tokens;
                        state.initialized = true;
                        state.serialize(&mut accounts[1].userdata);
                        Ok(())
                    }
                }
            }
            Instruction::ApplyTimestamp(dt) => {
                let mut state = Self::deserialize(&accounts[1].userdata).unwrap();
                if !state.is_pending() {
                    Err(BudgetError::ContractNotPending(tx.keys[1]))
                } else if !state.initialized {
                    trace!("contract is uninitialized");
                    Err(BudgetError::UninitializedContract(tx.keys[1]))
                } else {
                    trace!("apply timestamp");
                    state.apply_timestamp(&tx.keys, accounts, *dt)?;
                    trace!("apply timestamp committed");
                    state.serialize(&mut accounts[1].userdata);
                    Ok(())
                }
            }
            Instruction::ApplySignature => {
                let mut state = Self::deserialize(&accounts[1].userdata).unwrap();
                if !state.is_pending() {
                    Err(BudgetError::ContractNotPending(tx.keys[1]))
                } else if !state.initialized {
                    trace!("contract is uninitialized");
                    Err(BudgetError::UninitializedContract(tx.keys[1]))
                } else {
                    trace!("apply signature");
                    state.apply_signature(&tx.keys, accounts)?;
                    trace!("apply signature committed");
                    state.serialize(&mut accounts[1].userdata);
                    Ok(())
                }
            }
            Instruction::NewVote(_vote) => {
                // TODO: move vote instruction into a different contract
                trace!("GOT VOTE! last_id={}", tx.last_id);
                Ok(())
            }
        }
    }
    fn serialize(&self, output: &mut [u8]) {
        let len = serialized_size(self).unwrap() as u64;
        {
            let writer = io::BufWriter::new(&mut output[..8]);
            serialize_into(writer, &len).unwrap();
        }
        {
            let writer = io::BufWriter::new(&mut output[8..8 + len as usize]);
            serialize_into(writer, self).unwrap();
        }
    }

    pub fn deserialize(input: &[u8]) -> bincode::Result<Self> {
        if input.len() < 8 {
            return Err(Box::new(bincode::ErrorKind::SizeLimit));
        }
        let len: u64 = deserialize(&input[..8]).unwrap();
        if len < 3 {
            return Err(Box::new(bincode::ErrorKind::SizeLimit));
        }
        if input.len() < 8 + len as usize {
            return Err(Box::new(bincode::ErrorKind::SizeLimit));
        }
        deserialize(&input[8..8 + len as usize])
    }

    fn save_error_to_budget_state(e: BudgetError, accounts: &mut [Account]) -> () {
        if let Ok(mut state) = BudgetContract::deserialize(&accounts[1].userdata) {
            trace!("saved error {:?}", e);
            state.last_error = Some(e);
            state.serialize(&mut accounts[1].userdata);
        } else {
            trace!("error in uninitialized contract {:?}", e,);
        }
    }

    /// Budget DSL contract interface
    /// * tx - the transaction
    /// * accounts[0] - The source of the tokens
    /// * accounts[1] - The contract context.  Once the contract has been completed, the tokens can
    /// be spent from this account .
    pub fn process_transaction(tx: &Transaction, accounts: &mut [Account]) -> () {
        let instruction = deserialize(&tx.userdata).unwrap();
        let _ = Self::apply_debits_to_budget_state(tx, accounts, &instruction)
            .and_then(|_| Self::apply_credits_to_budget_state(tx, accounts, &instruction))
            .map_err(|e| {
                trace!("saving error {:?}", e);
                Self::save_error_to_budget_state(e, accounts);
            });
    }

    //TODO the contract needs to provide a "get_balance" introspection call of the userdata
    pub fn get_balance(account: &Account) -> i64 {
        if let Ok(state) = deserialize(&account.userdata) {
            let state: BudgetContract = state;
            if state.is_pending() {
                0
            } else {
                account.tokens
            }
        } else {
            account.tokens
        }
    }
}
#[cfg(test)]
mod test {
    use bank::Account;
    use bincode::serialize;
    use budget_contract::{BudgetContract, BudgetError};
    use chrono::prelude::{DateTime, NaiveDate, Utc};
    use hash::Hash;
    use signature::{GenKeys, Keypair, KeypairUtil, Pubkey};
    use transaction::Transaction;
    #[test]
    fn test_serializer() {
        let mut a = Account::new(0, 512, BudgetContract::id());
        let b = BudgetContract::default();
        b.serialize(&mut a.userdata);
        let buf = serialize(&b).unwrap();
        assert_eq!(a.userdata[8..8 + buf.len()], buf[0..]);
        let c = BudgetContract::deserialize(&a.userdata).unwrap();
        assert_eq!(b, c);
    }

    #[test]
    fn test_transfer_on_date() {
        let mut accounts = vec![
            Account::new(1, 0, BudgetContract::id()),
            Account::new(0, 512, BudgetContract::id()),
            Account::new(0, 0, BudgetContract::id()),
        ];
        let from_account = 0;
        let contract_account = 1;
        let to_account = 2;
        let from = Keypair::new();
        let contract = Keypair::new();
        let to = Keypair::new();
        let rando = Keypair::new();
        let dt = Utc::now();
        let tx = Transaction::budget_new_on_date(
            &from,
            to.pubkey(),
            contract.pubkey(),
            dt,
            1,
            Hash::default(),
        );
        BudgetContract::process_transaction(&tx, &mut accounts);
        assert_eq!(accounts[from_account].tokens, 0);
        assert_eq!(accounts[contract_account].tokens, 1);
        let state = BudgetContract::deserialize(&accounts[contract_account].userdata).unwrap();
        assert_eq!(state.last_error, None);
        assert!(state.is_pending());

        // Attack! Try to payout to a rando key
        let tx = Transaction::budget_new_timestamp(
            &from,
            contract.pubkey(),
            rando.pubkey(),
            dt,
            Hash::default(),
        );
        BudgetContract::process_transaction(&tx, &mut accounts);
        assert_eq!(accounts[from_account].tokens, 0);
        assert_eq!(accounts[contract_account].tokens, 1);
        assert_eq!(accounts[to_account].tokens, 0);

        let state = BudgetContract::deserialize(&accounts[contract_account].userdata).unwrap();
        assert_eq!(
            state.last_error,
            Some(BudgetError::DestinationMissing(to.pubkey()))
        );
        assert!(state.is_pending());

        // Now, acknowledge the time in the condition occurred and
        // that pubkey's funds are now available.
        let tx = Transaction::budget_new_timestamp(
            &from,
            contract.pubkey(),
            to.pubkey(),
            dt,
            Hash::default(),
        );
        BudgetContract::process_transaction(&tx, &mut accounts);
        assert_eq!(accounts[from_account].tokens, 0);
        assert_eq!(accounts[contract_account].tokens, 0);
        assert_eq!(accounts[to_account].tokens, 1);

        let state = BudgetContract::deserialize(&accounts[contract_account].userdata).unwrap();
        assert!(!state.is_pending());

        // try to replay the timestamp contract
        BudgetContract::process_transaction(&tx, &mut accounts);
        assert_eq!(accounts[from_account].tokens, 0);
        assert_eq!(accounts[contract_account].tokens, 0);
        assert_eq!(accounts[to_account].tokens, 1);
        let state = BudgetContract::deserialize(&accounts[contract_account].userdata).unwrap();
        assert_eq!(
            state.last_error,
            Some(BudgetError::ContractNotPending(contract.pubkey()))
        );
    }
    #[test]
    fn test_cancel_transfer() {
        let mut accounts = vec![
            Account::new(1, 0, BudgetContract::id()),
            Account::new(0, 512, BudgetContract::id()),
            Account::new(0, 0, BudgetContract::id()),
        ];
        let from_account = 0;
        let contract_account = 1;
        let pay_account = 2;
        let from = Keypair::new();
        let contract = Keypair::new();
        let to = Keypair::new();
        let dt = Utc::now();
        let tx = Transaction::budget_new_on_date(
            &from,
            to.pubkey(),
            contract.pubkey(),
            dt,
            1,
            Hash::default(),
        );
        BudgetContract::process_transaction(&tx, &mut accounts);
        assert_eq!(accounts[from_account].tokens, 0);
        assert_eq!(accounts[contract_account].tokens, 1);
        let state = BudgetContract::deserialize(&accounts[contract_account].userdata).unwrap();
        assert_eq!(state.last_error, None);
        assert!(state.is_pending());

        // Attack! try to put the tokens into the wrong account with cancel
        let tx =
            Transaction::budget_new_signature(&to, contract.pubkey(), to.pubkey(), Hash::default());
        // unit test hack, the `from account` is passed instead of the `to` account to avoid
        // creating more account vectors
        BudgetContract::process_transaction(&tx, &mut accounts);
        // nothing should be changed because apply witness didn't finalize a payment
        assert_eq!(accounts[from_account].tokens, 0);
        assert_eq!(accounts[contract_account].tokens, 1);
        // this would be the `to.pubkey()` account
        assert_eq!(accounts[pay_account].tokens, 0);

        // Now, cancel the transaction. from gets her funds back
        let tx = Transaction::budget_new_signature(
            &from,
            contract.pubkey(),
            from.pubkey(),
            Hash::default(),
        );
        BudgetContract::process_transaction(&tx, &mut accounts);
        assert_eq!(accounts[from_account].tokens, 0);
        assert_eq!(accounts[contract_account].tokens, 0);
        assert_eq!(accounts[pay_account].tokens, 1);

        // try to replay the signature contract
        let tx = Transaction::budget_new_signature(
            &from,
            contract.pubkey(),
            from.pubkey(),
            Hash::default(),
        );
        BudgetContract::process_transaction(&tx, &mut accounts);
        assert_eq!(accounts[from_account].tokens, 0);
        assert_eq!(accounts[contract_account].tokens, 0);
        assert_eq!(accounts[pay_account].tokens, 1);

        let state = BudgetContract::deserialize(&accounts[contract_account].userdata).unwrap();
        assert_eq!(
            state.last_error,
            Some(BudgetError::ContractNotPending(contract.pubkey()))
        );
    }

    /// Detect binary changes in the serialized contract userdata, which could have a downstream
    /// affect on SDKs and DApps
    #[test]
    fn test_sdk_serialize() {
        let keypair = &GenKeys::new([0u8; 32]).gen_n_keypairs(1)[0];
        let to = Pubkey::new(&[
            1, 1, 1, 4, 5, 6, 7, 8, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, 8, 7, 6, 5, 4,
            1, 1, 1,
        ]);
        let contract = Pubkey::new(&[
            2, 2, 2, 4, 5, 6, 7, 8, 9, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 9, 8, 7, 6, 5, 4,
            2, 2, 2,
        ]);
        let date =
            DateTime::<Utc>::from_utc(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11), Utc);
        let date_is_08601 = "2016-07-08T09:10:11Z";

        let tx = Transaction::budget_new(&keypair, to, 192, Hash::default());
        assert_eq!(
            tx.userdata,
            vec![
                0, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 192, 0, 0, 0, 0, 0,
                0, 0, 1, 1, 1, 4, 5, 6, 7, 8, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, 8, 7,
                6, 5, 4, 1, 1, 1
            ]
        );

        let tx =
            Transaction::budget_new_on_date(&keypair, to, contract, date, 192, Hash::default());
        assert_eq!(
            tx.userdata,
            vec![
                0, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 20, 0, 0,
                0, 0, 0, 0, 0, 50, 48, 49, 54, 45, 48, 55, 45, 48, 56, 84, 48, 57, 58, 49, 48, 58,
                49, 49, 90, 32, 253, 186, 201, 177, 11, 117, 135, 187, 167, 181, 188, 22, 59, 206,
                105, 231, 150, 215, 30, 78, 212, 76, 16, 252, 180, 72, 134, 137, 247, 161, 68, 192,
                0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 4, 5, 6, 7, 8, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 9, 8, 7, 6, 5, 4, 1, 1, 1, 1, 0, 0, 0, 32, 253, 186, 201, 177, 11, 117, 135,
                187, 167, 181, 188, 22, 59, 206, 105, 231, 150, 215, 30, 78, 212, 76, 16, 252, 180,
                72, 134, 137, 247, 161, 68, 192, 0, 0, 0, 0, 0, 0, 0, 32, 253, 186, 201, 177, 11,
                117, 135, 187, 167, 181, 188, 22, 59, 206, 105, 231, 150, 215, 30, 78, 212, 76, 16,
                252, 180, 72, 134, 137, 247, 161, 68
            ]
        );

        // ApplyTimestamp(date)
        let tx = Transaction::budget_new_timestamp(
            &keypair,
            keypair.pubkey(),
            to,
            date,
            Hash::default(),
        );
        let mut expected_userdata = vec![1, 0, 0, 0, 20, 0, 0, 0, 0, 0, 0, 0];
        expected_userdata.extend(date_is_08601.as_bytes());
        assert_eq!(tx.userdata, expected_userdata,);

        // ApplySignature
        let tx = Transaction::budget_new_signature(&keypair, keypair.pubkey(), to, Hash::default());
        assert_eq!(tx.userdata, vec![2, 0, 0, 0]);
    }
}
