use candid::{CandidType, Nat, Principal};
use ic_cdk::trap;
use ic_ethereum_types::Address;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::DefaultMemoryImpl;
use ic_stable_structures::{storable::Bound, Cell, Storable};
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cell::RefCell;

use std::collections::BTreeMap;
use std::str::FromStr;

use crate::endpoints::{
    AddEvmToIcpTx, AddIcpToEvmTx, CandidEvmToIcp, CandidIcpToEvm, InitArgs, MinterArgs, TokenPair,
    Transaction,
};
use crate::guard::TaskType;
use crate::scrape_events::NATIVE_ERC20_ADDRESS;

use std::collections::HashSet;
use std::fmt::Debug;

use crate::minter_clinet::appic_minter_types::events::{TransactionReceipt, TransactionStatus};

#[derive(Clone, CandidType, PartialEq, PartialOrd, Eq, Ord, Debug, Deserialize, Serialize)]
pub enum Oprator {
    DfinityCkEthMinter,
    AppicMinter,
}

#[derive(Clone, PartialEq, Ord, PartialOrd, Eq, Debug, Deserialize, Serialize)]
pub struct Minter {
    pub id: Principal,
    pub last_observed_event: u64,
    pub last_scraped_event: u64,
    pub oprator: Oprator,
    pub evm_to_icp_fee: Nat,
    pub icp_to_evm_fee: Nat,
    pub chain_id: ChainId,
}

impl Minter {
    pub fn update_last_observed_event(&mut self, event: u64) {
        self.last_observed_event = event
    }

    pub fn update_last_scraped_event(&mut self, event: u64) {
        self.last_scraped_event = event
    }

    pub fn from_minter_args(args: &MinterArgs) -> Self {
        let MinterArgs {
            chain_id,
            minter_id,
            oprator,
            last_observed_event,
            last_scraped_event,
            evm_to_icp_fee,
            icp_to_evm_fee,
        } = args.clone();
        Self {
            id: minter_id,
            last_observed_event: nat_to_u64(last_observed_event),
            last_scraped_event: nat_to_u64(last_scraped_event),
            oprator,
            evm_to_icp_fee,
            icp_to_evm_fee,
            chain_id: ChainId::from(chain_id),
        }
    }
}

#[derive(Clone, PartialEq, Ord, Eq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct MinterKey(pub ChainId, pub Oprator);

impl MinterKey {
    pub fn oprator(&self) -> Oprator {
        self.1.clone()
    }

    pub fn chain_id(&self) -> ChainId {
        self.0.clone()
    }

    pub fn from_minter_args(args: &MinterArgs) -> Self {
        Self(args.chain_id.clone().into(), args.oprator.clone())
    }
}

impl From<&Minter> for MinterKey {
    fn from(value: &Minter) -> Self {
        Self(value.chain_id.clone(), value.oprator.clone())
    }
}

type TransactionHash = String;

#[derive(Clone, PartialEq, Ord, Eq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct EvmToIcpTxIdentifier(TransactionHash, ChainId);

impl EvmToIcpTxIdentifier {
    pub fn new(transaction_hash: &TransactionHash, chain_id: &ChainId) -> Self {
        EvmToIcpTxIdentifier(transaction_hash.clone(), chain_id.clone())
    }
}
impl From<&AddEvmToIcpTx> for EvmToIcpTxIdentifier {
    fn from(value: &AddEvmToIcpTx) -> Self {
        Self::new(
            &value.transaction_hash,
            &ChainId::from(value.chain_id.clone()),
        )
    }
}

#[derive(Clone, CandidType, PartialEq, Ord, Eq, PartialOrd, Debug, Deserialize, Serialize)]
pub enum EvmToIcpStatus {
    PendingVerification,
    Accepted,
    Minted,
    Invalid(String),
    Quarantined,
}

#[derive(Clone, PartialEq, Ord, Eq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct EvmToIcpTx {
    pub from_address: Address,
    pub transaction_hash: TransactionHash,
    pub value: Nat,
    pub block_number: Option<Nat>,
    pub actual_received: Option<Nat>,
    pub principal: Principal,
    pub subaccount: Option<[u8; 32]>,
    pub chain_id: ChainId,
    pub total_gas_spent: Option<Nat>,
    pub erc20_contract_address: Address,
    pub icrc_ledger_id: Option<Principal>,
    pub status: EvmToIcpStatus,
    pub verified: bool,
    pub time: u64,
    pub oprator: Oprator,
}

pub type NativeLedgerBurnIndex = Nat;

#[derive(Clone, PartialEq, Ord, Eq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct IcpToEvmIdentifier(NativeLedgerBurnIndex, ChainId);
impl IcpToEvmIdentifier {
    pub fn new(native_ledger_burn_index: &NativeLedgerBurnIndex, chain_id: &ChainId) -> Self {
        IcpToEvmIdentifier(native_ledger_burn_index.clone(), chain_id.clone())
    }
}

impl From<&AddIcpToEvmTx> for IcpToEvmIdentifier {
    fn from(value: &AddIcpToEvmTx) -> Self {
        Self::new(
            &value.native_ledger_burn_index,
            &ChainId::from(value.chain_id.clone()),
        )
    }
}

#[derive(CandidType, Clone, PartialEq, Ord, Eq, PartialOrd, Debug, Deserialize, Serialize)]
pub enum IcpToEvmStatus {
    PendingVerification,
    Accepted,
    Created,
    SignedTransaction,
    FinalizedTransaction,
    ReplacedTransaction,
    Reimbursed,
    QuarantinedReimbursement,
    Successful,
    Failed,
}

#[derive(Clone, PartialEq, Ord, Eq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct IcpToEvmTx {
    pub transaction_hash: Option<TransactionHash>,
    pub native_ledger_burn_index: NativeLedgerBurnIndex,
    pub withdrawal_amount: Nat,
    pub actual_received: Option<Nat>,
    pub destination: Address,
    pub from: Principal,
    pub chain_id: ChainId,
    pub from_subaccount: Option<[u8; 32]>,
    pub time: u64,
    pub max_transaction_fee: Option<Nat>,
    pub effective_gas_price: Option<Nat>,
    pub gas_used: Option<Nat>,
    pub toatal_gas_spent: Option<Nat>,
    pub erc20_ledger_burn_index: Option<Nat>,
    pub erc20_contract_address: Address,
    pub icrc_ledger_id: Option<Principal>,
    pub verified: bool,
    pub status: IcpToEvmStatus,
    pub oprator: Oprator,
}

#[derive(Clone, PartialEq, Ord, Eq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Erc20Identifier(pub Address, pub ChainId);

impl Erc20Identifier {
    pub fn new(contract: &Address, chain_id: &ChainId) -> Self {
        Self(contract.clone(), chain_id.clone())
    }

    pub fn erc20_address(&self) -> Address {
        self.0
    }
    pub fn chain_id(&self) -> ChainId {
        self.1
    }
}
// State Definition,
// All types of transactions will be sotred in this stable state
#[derive(Clone, PartialEq, Debug, Eq, Deserialize, Serialize)]
pub struct State {
    /// Locks preventing concurrent execution timer tasks
    pub active_tasks: HashSet<TaskType>,

    // List of all minters including (cketh dfinity and appic minters)
    pub minters: BTreeMap<MinterKey, Minter>,

    // List of all evm_to_icp transactions
    pub evm_to_icp_txs: BTreeMap<EvmToIcpTxIdentifier, EvmToIcpTx>,

    // list of all icp_to_evm transactions
    pub icp_to_evm_txs: BTreeMap<IcpToEvmIdentifier, IcpToEvmTx>,

    pub supported_ckerc20_tokens: BTreeMap<Erc20Identifier, Principal>,
    pub supported_twin_appic_tokens: BTreeMap<Erc20Identifier, Principal>,
}

impl State {
    pub fn get_minter(&self, minter_key: &MinterKey) -> Option<Minter> {
        match self.minters.get(minter_key) {
            Some(minter) => Some(minter.clone()),
            None => None,
        }
    }

    pub fn get_minter_mut(&mut self, minter_key: &MinterKey) -> Option<&mut Minter> {
        self.minters.get_mut(minter_key)
    }

    pub fn get_minters_iter(&self) -> std::collections::btree_map::IntoIter<MinterKey, Minter> {
        self.minters.clone().into_iter()
    }

    pub fn if_chain_id_exists(&self, chain_id: &ChainId) -> bool {
        let chain_ids: Vec<ChainId> = self
            .get_minters_iter()
            .map(|minter| minter.1.chain_id)
            .collect();

        chain_ids.contains(chain_id)
    }

    pub fn get_minters_mut_iter(
        &mut self,
    ) -> std::collections::btree_map::IterMut<'_, MinterKey, Minter> {
        self.minters.iter_mut()
    }

    pub fn record_minter(&mut self, minter: Minter) {
        self.minters.insert(MinterKey::from(&minter), minter);
    }

    pub fn get_icrc_twin_for_erc20(
        &self,
        erc20_identifier: &Erc20Identifier,
        oprator: &Oprator,
    ) -> Option<Principal> {
        match oprator {
            Oprator::AppicMinter => self
                .supported_twin_appic_tokens
                .get(erc20_identifier)
                .map(|token_principal| token_principal.clone()),
            Oprator::DfinityCkEthMinter => self
                .supported_ckerc20_tokens
                .get(erc20_identifier)
                .map(|token_principal| token_principal.clone()),
        }
    }

    pub fn if_evm_to_icp_tx_exists(&self, identifier: &EvmToIcpTxIdentifier) -> bool {
        self.evm_to_icp_txs.get(identifier).is_some()
    }

    pub fn if_icp_to_evm_tx_exists(&self, identifier: &IcpToEvmIdentifier) -> bool {
        self.icp_to_evm_txs.get(identifier).is_some()
    }

    pub fn record_new_evm_to_icp(&mut self, identifier: EvmToIcpTxIdentifier, tx: EvmToIcpTx) {
        self.evm_to_icp_txs.insert(identifier, tx);
    }

    pub fn record_accepted_evm_to_icp(
        &mut self,
        identifier: &EvmToIcpTxIdentifier,
        transaction_hash: TransactionHash,
        block_number: Nat,
        from_address: String,
        value: Nat,
        principal: Principal,
        erc20_contract_address: String,
        subaccount: Option<[u8; 32]>,
        chain_id: &ChainId,
        oprator: &Oprator,
        timestamp: u64,
    ) {
        if let Some(tx) = self.evm_to_icp_txs.get_mut(identifier) {
            *tx = EvmToIcpTx {
                verified: true,
                block_number: Some(block_number),
                from_address: Address::from_str(&from_address)
                    .expect("Should not fail converting minter address to Address"),
                value: value.clone(),
                principal,
                erc20_contract_address: Address::from_str(&erc20_contract_address)
                    .expect("Should not fail converting minter address to Address"),
                subaccount,
                status: EvmToIcpStatus::Accepted,
                ..tx.clone() // Copies the remaining fields
            };
        } else {
            self.record_new_evm_to_icp(
                identifier.clone(),
                EvmToIcpTx {
                    from_address: Address::from_str(&from_address)
                        .expect("Should not fail converting minter address to Address"),
                    transaction_hash,
                    value: value.clone(),
                    block_number: Some(block_number),
                    actual_received: None,
                    principal,
                    subaccount,
                    chain_id: chain_id.clone(),
                    total_gas_spent: None,
                    erc20_contract_address: Address::from_str(&erc20_contract_address)
                        .expect("Should not fail converting minter address to Address"),
                    icrc_ledger_id: self.get_icrc_twin_for_erc20(
                        &Erc20Identifier(
                            Address::from_str(&erc20_contract_address)
                                .expect("Should not fail converting minter address to Address"),
                            chain_id.clone(),
                        ),
                        oprator,
                    ),
                    status: EvmToIcpStatus::Accepted,
                    verified: true,
                    time: timestamp,
                    oprator: oprator.clone(),
                },
            );
        }
    }

    pub fn record_minted_evm_to_icp(
        &mut self,
        identifier: &EvmToIcpTxIdentifier,
        erc20_contract_address: String,
        evm_to_icp_fee: Nat,
    ) {
        if let Some(tx) = self.evm_to_icp_txs.get_mut(identifier) {
            // Fee calulation
            let actual_received: Option<Nat> = match is_native_token(
                &Address::from_str(&erc20_contract_address)
                    .expect("Should not fail converting minter address to Address"),
            ) {
                true => Some(tx.value.clone() - evm_to_icp_fee),
                false => Some(tx.value.clone()),
            };

            *tx = EvmToIcpTx {
                actual_received,
                erc20_contract_address: Address::from_str(&erc20_contract_address)
                    .expect("Should not fail converting minter address to Address"),
                status: EvmToIcpStatus::Minted,
                ..tx.clone() // Copies the remaining fields
            };
        }
    }

    pub fn record_invalid_evm_to_icp(&mut self, identifier: &EvmToIcpTxIdentifier, reason: String) {
        if let Some(tx) = self.evm_to_icp_txs.get_mut(identifier) {
            *tx = EvmToIcpTx {
                status: EvmToIcpStatus::Invalid(reason),
                ..tx.clone() // Copies the remaining fields
            };
        }
    }

    pub fn record_quarantined_evm_to_icp(&mut self, identifier: &EvmToIcpTxIdentifier) {
        if let Some(tx) = self.evm_to_icp_txs.get_mut(identifier) {
            *tx = EvmToIcpTx {
                status: EvmToIcpStatus::Quarantined,
                ..tx.clone() // Copies the remaining fields
            };
        }
    }

    pub fn record_new_icp_to_evm(&mut self, identifier: IcpToEvmIdentifier, tx: IcpToEvmTx) {
        self.icp_to_evm_txs.insert(identifier, tx);
    }

    pub fn record_accepted_icp_to_evm(
        &mut self,
        identifier: &IcpToEvmIdentifier,
        max_transaction_fee: Option<Nat>,
        withdrawal_amount: Nat,
        erc20_contract_address: String,
        destination: String,
        native_ledger_burn_index: Nat,
        erc20_ledger_burn_index: Option<Nat>,
        from: Principal,
        from_subaccount: Option<[u8; 32]>,
        created_at: Option<u64>,
        oprator: Oprator,
        chain_id: ChainId,
        timestamp: u64,
    ) {
        if let Some(tx) = self.icp_to_evm_txs.get_mut(identifier) {
            *tx = IcpToEvmTx {
                verified: true,
                max_transaction_fee,
                withdrawal_amount,
                erc20_contract_address: Address::from_str(&erc20_contract_address)
                    .expect("Should not fail converting minter address to Address"),
                destination: Address::from_str(&destination)
                    .expect("Should not fail converting minter address to Address"),
                native_ledger_burn_index,
                erc20_ledger_burn_index,
                from,
                from_subaccount,
                status: IcpToEvmStatus::Accepted,
                ..tx.clone()
            }
        } else {
            let new_tx = IcpToEvmTx {
                native_ledger_burn_index,
                withdrawal_amount,
                actual_received: None,
                destination: Address::from_str(&destination)
                    .expect("Should not fail converting minter address to Address"),
                from,
                from_subaccount,
                time: created_at.unwrap_or(timestamp),
                max_transaction_fee,
                erc20_ledger_burn_index,
                icrc_ledger_id: self.get_icrc_twin_for_erc20(
                    &Erc20Identifier(
                        Address::from_str(&erc20_contract_address)
                            .expect("Should not fail converting minter address to Address"),
                        chain_id.clone(),
                    ),
                    &oprator,
                ),
                chain_id,
                erc20_contract_address: Address::from_str(&erc20_contract_address)
                    .expect("Should not fail converting minter address to Address"),
                verified: true,
                status: IcpToEvmStatus::Accepted,
                oprator,
                effective_gas_price: None,
                gas_used: None,
                toatal_gas_spent: None,
                transaction_hash: None,
            };

            self.record_new_icp_to_evm(identifier.clone(), new_tx);
        }
    }

    pub fn record_created_icp_to_evm(&mut self, identifier: &IcpToEvmIdentifier) {
        if let Some(tx) = self.icp_to_evm_txs.get_mut(identifier) {
            *tx = IcpToEvmTx {
                status: IcpToEvmStatus::Created,
                ..tx.clone()
            }
        }
    }

    pub fn record_signed_icp_to_evm(&mut self, identifier: &IcpToEvmIdentifier) {
        if let Some(tx) = self.icp_to_evm_txs.get_mut(identifier) {
            *tx = IcpToEvmTx {
                status: IcpToEvmStatus::SignedTransaction,
                ..tx.clone()
            }
        }
    }

    pub fn record_replaced_icp_to_evm(&mut self, identifier: &IcpToEvmIdentifier) {
        if let Some(tx) = self.icp_to_evm_txs.get_mut(identifier) {
            *tx = IcpToEvmTx {
                status: IcpToEvmStatus::ReplacedTransaction,
                ..tx.clone()
            }
        }
    }

    pub fn record_finalized_icp_to_evm(
        &mut self,
        identifier: &IcpToEvmIdentifier,
        receipt: TransactionReceipt,
        icp_to_evm_fee: Nat,
    ) {
        if let Some(tx) = self.icp_to_evm_txs.get_mut(identifier) {
            let actual_received: Option<Nat> = match is_native_token(&tx.erc20_contract_address) {
                true => Some(
                    tx.withdrawal_amount.clone()
                        - (receipt.gas_used.clone() * receipt.effective_gas_price.clone())
                        - icp_to_evm_fee.clone(),
                ),
                false => Some(tx.withdrawal_amount.clone()),
            };

            let status = match receipt.status {
                TransactionStatus::Success => IcpToEvmStatus::Successful,
                TransactionStatus::Failure => IcpToEvmStatus::Failed,
            };
            *tx = IcpToEvmTx {
                status,
                actual_received,
                transaction_hash: Some(receipt.transaction_hash),
                gas_used: Some(receipt.gas_used.clone()),
                effective_gas_price: Some(receipt.effective_gas_price.clone()),
                toatal_gas_spent: Some(
                    (receipt.gas_used * receipt.effective_gas_price) + icp_to_evm_fee,
                ),
                ..tx.clone()
            }
        }
    }

    pub fn record_reimbursed_icp_to_evm(&mut self, identifier: &IcpToEvmIdentifier) {
        if let Some(tx) = self.icp_to_evm_txs.get_mut(identifier) {
            *tx = IcpToEvmTx {
                status: IcpToEvmStatus::Reimbursed,
                ..tx.clone()
            }
        }
    }

    pub fn record_quarantined_reimbursed_icp_to_evm(&mut self, identifier: &IcpToEvmIdentifier) {
        if let Some(tx) = self.icp_to_evm_txs.get_mut(identifier) {
            *tx = IcpToEvmTx {
                status: IcpToEvmStatus::QuarantinedReimbursement,
                ..tx.clone()
            }
        }
    }

    pub fn all_icp_to_evm_iter(
        &self,
    ) -> std::collections::btree_map::Iter<'_, IcpToEvmIdentifier, IcpToEvmTx> {
        self.icp_to_evm_txs.iter()
    }

    pub fn all_unverified_icp_to_evm(&self) -> Vec<(IcpToEvmIdentifier, IcpToEvmTx)> {
        self.icp_to_evm_txs
            .iter()
            .filter(|(_identifier, tx)| tx.verified == false)
            .map(|(_identifier, tx)| (_identifier.clone(), tx.clone()))
            .collect()
    }

    pub fn remove_unverified_icp_to_evm(&mut self, identifier: &IcpToEvmIdentifier) {
        self.icp_to_evm_txs.remove(identifier);
    }

    pub fn all_evm_to_icp_iter(
        &self,
    ) -> std::collections::btree_map::Iter<'_, EvmToIcpTxIdentifier, EvmToIcpTx> {
        self.evm_to_icp_txs.iter()
    }

    pub fn all_unverified_evm_to_icp(&self) -> Vec<(EvmToIcpTxIdentifier, EvmToIcpTx)> {
        self.evm_to_icp_txs
            .iter()
            .filter(|(_identifier, tx)| tx.verified == false)
            .map(|(_identifier, tx)| (_identifier.clone(), tx.clone()))
            .collect()
    }

    pub fn remove_unverified_evm_to_icp(&mut self, identifier: &EvmToIcpTxIdentifier) {
        self.evm_to_icp_txs.remove(identifier);
    }

    pub fn get_transaction_for_address(&self, address: Address) -> Vec<Transaction> {
        let all_tx: Vec<Transaction> = self
            .all_evm_to_icp_iter()
            .filter_map(|(_id, tx)| {
                if tx.from_address == address {
                    Some(Transaction::from(CandidEvmToIcp::from(tx.clone())))
                } else {
                    None
                }
            })
            .chain(self.all_icp_to_evm_iter().filter_map(|(_id, tx)| {
                if tx.destination == address {
                    Some(Transaction::from(CandidIcpToEvm::from(tx.clone())))
                } else {
                    None
                }
            }))
            .collect();

        all_tx
    }

    pub fn get_transaction_for_principal(&self, principal_id: Principal) -> Vec<Transaction> {
        let all_tx: Vec<Transaction> = self
            .all_evm_to_icp_iter()
            .filter_map(|(_id, tx)| {
                if tx.principal == principal_id {
                    Some(Transaction::from(CandidEvmToIcp::from(tx.clone())))
                } else {
                    None
                }
            })
            .chain(self.all_icp_to_evm_iter().filter_map(|(_id, tx)| {
                if tx.from == principal_id {
                    Some(Transaction::from(CandidIcpToEvm::from(tx.clone())))
                } else {
                    None
                }
            }))
            .collect();

        all_tx
    }

    pub fn get_suported_twin_token_pairs(&self) -> Vec<TokenPair> {
        self.supported_ckerc20_tokens
            .iter()
            .map(|(erc20_identifier, ledger_id)| TokenPair {
                erc20_address: erc20_identifier.erc20_address().to_string(),
                ledger_id: *ledger_id,
                oprator: Oprator::DfinityCkEthMinter,
                chain_id: erc20_identifier.chain_id().into(),
            })
            .chain(
                self.supported_twin_appic_tokens
                    .iter()
                    .map(|(erc20_identifier, ledger_id)| TokenPair {
                        erc20_address: erc20_identifier.erc20_address().to_string(),
                        ledger_id: *ledger_id,
                        oprator: Oprator::AppicMinter,
                        chain_id: erc20_identifier.chain_id().into(),
                    }),
            )
            .collect()
    }
}

pub fn read_state<R>(f: impl FnOnce(&State) -> R) -> R {
    STATE.with(|cell| f(cell.borrow().get().expect_initialized()))
}

/// Mutates (part of) the current state using `f`.
///
/// Panics if there is no state.
pub fn mutate_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut State) -> R,
{
    STATE.with(|cell| {
        let mut borrowed = cell.borrow_mut();
        let mut state = borrowed.get().expect_initialized().clone();
        let result = f(&mut state);
        borrowed
            .set(ConfigState::Initialized(state))
            .expect("failed to write state in stable cell");
        result
    })
}

pub fn init_state(state: State) {
    STATE.with(|cell| {
        let mut borrowed = cell.borrow_mut();
        assert_eq!(
            borrowed.get(),
            &ConfigState::Uninitialized,
            "BUG: State is already initialized and has value {:?}",
            borrowed.get()
        );
        borrowed
            .set(ConfigState::Initialized(state))
            .expect("failed to initialize state in stable cell")
    });
}

impl From<InitArgs> for State {
    fn from(value: InitArgs) -> Self {
        let minters = BTreeMap::from_iter(value.minters.iter().map(|minter| {
            (
                MinterKey::from_minter_args(&minter),
                Minter::from_minter_args(&minter),
            )
        }));
        Self {
            active_tasks: Default::default(),
            minters,
            evm_to_icp_txs: Default::default(),
            icp_to_evm_txs: Default::default(),
            supported_ckerc20_tokens: Default::default(),
            supported_twin_appic_tokens: Default::default(),
        }
    }
}

impl From<Nat> for ChainId {
    fn from(value: Nat) -> Self {
        Self(value.0.to_u64().unwrap())
    }
}

impl From<ChainId> for Nat {
    fn from(value: ChainId) -> Self {
        Nat::from(value.0)
    }
}

pub fn nat_to_u64(value: Nat) -> u64 {
    value.0.to_u64().unwrap()
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct ChainId(pub u64);

impl AsRef<u64> for ChainId {
    fn as_ref(&self) -> &u64 {
        &self.0
    }
}

// State configuration
pub type StableMemory = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

}

const STATE_MEMORY_ID: MemoryId = MemoryId::new(0);

pub fn state_memory() -> StableMemory {
    MEMORY_MANAGER.with(|m| m.borrow().get(STATE_MEMORY_ID))
}

thread_local! {
    pub static STATE: RefCell<Cell<ConfigState, StableMemory>> = RefCell::new(Cell::init(
   state_memory(), ConfigState::default())
    .expect("failed to initialize stable cell for state"));
}

/// Configuration state of the lsm.
#[derive(Clone, PartialEq, Debug, Default)]
enum ConfigState {
    #[default]
    Uninitialized,
    // This state is only used between wasm module initialization and init().
    Initialized(State),
}

impl ConfigState {
    fn expect_initialized(&self) -> &State {
        match &self {
            ConfigState::Uninitialized => trap("BUG: state not initialized"),
            ConfigState::Initialized(s) => s,
        }
    }
}

impl Storable for ConfigState {
    fn to_bytes(&self) -> Cow<[u8]> {
        match &self {
            ConfigState::Uninitialized => Cow::Borrowed(&[]),
            ConfigState::Initialized(config) => Cow::Owned(encode(config)),
        }
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        if bytes.is_empty() {
            return ConfigState::Uninitialized;
        }
        ConfigState::Initialized(decode(bytes.as_ref()))
    }

    const BOUND: Bound = Bound::Unbounded;
}

fn encode<S: ?Sized + serde::Serialize>(state: &S) -> Vec<u8> {
    let mut buf = vec![];
    ciborium::ser::into_writer(state, &mut buf).expect("failed to encode state");
    buf
}

fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> T {
    ciborium::de::from_reader(bytes)
        .unwrap_or_else(|e| panic!("failed to decode state bytes {}: {e}", hex::encode(bytes)))
}

pub fn is_native_token(address: &Address) -> bool {
    address
        == &Address::from_str(NATIVE_ERC20_ADDRESS).expect("Should not fail converintg to address")
}
