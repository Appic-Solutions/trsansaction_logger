use candid::{Deserialize, Nat, Principal};
use serde::Serialize;

use crate::minter_clinet::appic_minter_types::events::Event as AppicEvent;

use crate::minter_clinet::dfinity_ck_minter_types::events::Event as DfintiyEvent;

use crate::minter_clinet::appic_minter_types::events::EventPayload as AppicEventPayload;
use crate::minter_clinet::dfinity_ck_minter_types::events::EventPayload as DfinityEventPayload;

use crate::minter_clinet::appic_minter_types::events::EventPayload::{
    AcceptedDeposit as AppicAcceptedDeposit, AcceptedErc20Deposit as AppicAcceptedErc20Deposit,
    AcceptedErc20WithdrawalRequest as AppicAcceptedErc20WithdrawalRequest,
    AcceptedNativeWithdrawalRequest as AppicAcceptedNativeWithdrawalRequest,
    CreatedTransaction as AppicCreatedTransaction,
    FailedErc20WithdrawalRequest as AppicFailedErc20WithdrawalRequest,
    FinalizedTransaction as AppicFinalizedTransaction, MintedErc20 as AppicMintedErc20,
    MintedNative as AppicMintedNative, ReimbursedErc20Withdrawal as AppicReimbursedErc20Withdrawal,
    ReimbursedNativeWithdrawal as AppicReimbursedNativeWithdrawal,
    ReplacedTransaction as AppicReplacedTransaction, SignedTransaction as AppicSignedTransaction,
};
use crate::minter_clinet::{AppicGetEventsResult, DfinityCkGetEventsResult};

use super::appic_minter_types::events::EventSource as AppicEventSource;

// standard type for events returned from minters
#[derive(Clone, PartialEq, Ord, Eq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Events {
    pub events: Vec<AppicEvent>,
}

// A trait for filtering and mapping EventResults form both appic and dfinity cketh minters into a Standard Event type
pub trait Reduce {
    fn reduce(self) -> Events;
}

impl Reduce for DfinityCkGetEventsResult {
    fn reduce(self) -> Events {
        Events {
            events: AppicGetEventsResult::from(self).events,
        }
    }
}

impl Reduce for AppicGetEventsResult {
    fn reduce(self) -> Events {
        let reduced: Vec<AppicEvent> = self
            .events
            .into_iter()
            .filter(|event| {
                matches!(
                    event.payload,
                    AppicEventPayload::AcceptedDeposit { .. }
                        | AppicEventPayload::AcceptedErc20Deposit { .. }
                        | AppicEventPayload::MintedNative { .. }
                        | AppicEventPayload::MintedErc20 { .. }
                        | AppicEventPayload::AcceptedNativeWithdrawalRequest { .. }
                        | AppicEventPayload::CreatedTransaction { .. }
                        | AppicEventPayload::SignedTransaction { .. }
                        | AppicEventPayload::ReplacedTransaction { .. }
                        | AppicEventPayload::FinalizedTransaction { .. }
                        | AppicEventPayload::ReimbursedNativeWithdrawal { .. }
                        | AppicEventPayload::ReimbursedErc20Withdrawal { .. }
                        | AppicEventPayload::AcceptedErc20WithdrawalRequest { .. }
                        | AppicEventPayload::FailedErc20WithdrawalRequest { .. }
                        | AppicEventPayload::InvalidDeposit { .. }
                        | AppicEventPayload::QuarantinedDeposit { .. }
                        | AppicEventPayload::QuarantinedReimbursement { .. }
                )
            })
            .collect();
        Events { events: reduced }
    }
}

impl From<DfinityCkGetEventsResult> for AppicGetEventsResult {
    fn from(value: DfinityCkGetEventsResult) -> AppicGetEventsResult {
        let filtered_mapped: Vec<AppicEvent> = value
            .events
            .iter()
            .filter_map(|event| {
                let timestamp = event.timestamp;
                let event_payload = match event.payload.clone() {
                    DfinityEventPayload::Init(_init_arg) => None,
                    DfinityEventPayload::Upgrade(_upgrade_arg) => None,
                    DfinityEventPayload::AcceptedDeposit {
                        transaction_hash,
                        block_number,
                        log_index,
                        from_address,
                        value,
                        principal,
                        subaccount,
                    } => Some(AppicEventPayload::AcceptedDeposit {
                        transaction_hash,
                        block_number,
                        log_index,
                        from_address,
                        value,
                        principal,
                        subaccount,
                    }),
                    DfinityEventPayload::AcceptedErc20Deposit {
                        transaction_hash,
                        block_number,
                        log_index,
                        from_address,
                        value,
                        principal,
                        erc20_contract_address,
                        subaccount,
                    } => Some(AppicEventPayload::AcceptedErc20Deposit {
                        transaction_hash,
                        block_number,
                        log_index,
                        from_address,
                        value,
                        principal,
                        erc20_contract_address,
                        subaccount,
                    }),
                    DfinityEventPayload::InvalidDeposit {
                        event_source,
                        reason,
                    } => Some(AppicEventPayload::InvalidDeposit {
                        event_source: AppicEventSource {
                            log_index: event_source.log_index,
                            transaction_hash: event_source.transaction_hash,
                        },
                        reason,
                    }),
                    DfinityEventPayload::MintedCkEth {
                        event_source,
                        mint_block_index,
                    } => Some(AppicEventPayload::MintedNative {
                        event_source: AppicEventSource {
                            log_index: event_source.log_index,
                            transaction_hash: event_source.transaction_hash,
                        },
                        mint_block_index,
                    }),
                    DfinityEventPayload::SyncedToBlock { block_number } => None,
                    DfinityEventPayload::SyncedErc20ToBlock { block_number } => None,
                    DfinityEventPayload::SyncedDepositWithSubaccountToBlock { block_number } => {
                        None
                    }
                    DfinityEventPayload::AcceptedEthWithdrawalRequest {
                        withdrawal_amount,
                        destination,
                        ledger_burn_index,
                        from,
                        from_subaccount,
                        created_at,
                    } => Some(AppicEventPayload::AcceptedNativeWithdrawalRequest {
                        withdrawal_amount,
                        destination,
                        ledger_burn_index,
                        from,
                        from_subaccount,
                        created_at,
                    }),
                    DfinityEventPayload::CreatedTransaction {
                        withdrawal_id,
                        transaction,
                    } => Some(AppicEventPayload::CreatedTransaction {
                        withdrawal_id,
                        transaction: transaction.into(),
                    }),
                    DfinityEventPayload::SignedTransaction {
                        withdrawal_id,
                        raw_transaction,
                    } => Some(AppicEventPayload::SignedTransaction {
                        withdrawal_id,
                        raw_transaction,
                    }),
                    DfinityEventPayload::ReplacedTransaction {
                        withdrawal_id,
                        transaction,
                    } => Some(AppicEventPayload::ReplacedTransaction {
                        withdrawal_id,
                        transaction: transaction.into(),
                    }),
                    DfinityEventPayload::FinalizedTransaction {
                        withdrawal_id,
                        transaction_receipt,
                    } => Some(AppicEventPayload::FinalizedTransaction {
                        withdrawal_id,
                        transaction_receipt: transaction_receipt.into(),
                    }),
                    DfinityEventPayload::ReimbursedEthWithdrawal {
                        reimbursed_in_block,
                        withdrawal_id,
                        reimbursed_amount,
                        transaction_hash,
                    } => Some(AppicEventPayload::ReimbursedNativeWithdrawal {
                        reimbursed_in_block,
                        withdrawal_id,
                        reimbursed_amount,
                        transaction_hash,
                    }),
                    DfinityEventPayload::ReimbursedErc20Withdrawal {
                        withdrawal_id,
                        burn_in_block,
                        reimbursed_in_block,
                        ledger_id,
                        reimbursed_amount,
                        transaction_hash,
                    } => Some(AppicEventPayload::ReimbursedErc20Withdrawal {
                        withdrawal_id,
                        burn_in_block,
                        reimbursed_in_block,
                        ledger_id,
                        reimbursed_amount,
                        transaction_hash,
                    }),
                    DfinityEventPayload::SkippedBlock {
                        contract_address,
                        block_number,
                    } => None,
                    DfinityEventPayload::AddedCkErc20Token {
                        chain_id,
                        address,
                        ckerc20_token_symbol,
                        ckerc20_ledger_id,
                    } => None,
                    DfinityEventPayload::AcceptedErc20WithdrawalRequest {
                        max_transaction_fee,
                        withdrawal_amount,
                        erc20_contract_address,
                        destination,
                        cketh_ledger_burn_index,
                        ckerc20_ledger_id,
                        ckerc20_ledger_burn_index,
                        from,
                        from_subaccount,
                        created_at,
                    } => Some(AppicEventPayload::AcceptedErc20WithdrawalRequest {
                        max_transaction_fee,
                        withdrawal_amount,
                        erc20_contract_address,
                        destination,
                        native_ledger_burn_index: cketh_ledger_burn_index,
                        erc20_ledger_id: ckerc20_ledger_id,
                        erc20_ledger_burn_index: ckerc20_ledger_burn_index,
                        from,
                        from_subaccount,
                        created_at,
                    }),
                    DfinityEventPayload::FailedErc20WithdrawalRequest {
                        withdrawal_id,
                        reimbursed_amount,
                        to,
                        to_subaccount,
                    } => Some(AppicEventPayload::FailedErc20WithdrawalRequest {
                        withdrawal_id,
                        reimbursed_amount,
                        to,
                        to_subaccount,
                    }),
                    DfinityEventPayload::MintedCkErc20 {
                        event_source,
                        mint_block_index,
                        ckerc20_token_symbol,
                        erc20_contract_address,
                    } => Some(AppicEventPayload::MintedErc20 {
                        event_source: AppicEventSource {
                            log_index: event_source.log_index,
                            transaction_hash: event_source.transaction_hash,
                        },
                        mint_block_index,
                        erc20_token_symbol: ckerc20_token_symbol,
                        erc20_contract_address,
                    }),
                    DfinityEventPayload::QuarantinedDeposit { event_source } => {
                        Some(AppicEventPayload::QuarantinedDeposit {
                            event_source: AppicEventSource {
                                log_index: event_source.log_index,
                                transaction_hash: event_source.transaction_hash,
                            },
                        })
                    }
                    DfinityEventPayload::QuarantinedReimbursement { index } => {
                        Some(AppicEventPayload::QuarantinedReimbursement {
                            index: index.into(),
                        })
                    }
                };
                match event_payload {
                    Some(e) => Some(AppicEvent {
                        timestamp,
                        payload: e,
                    }),
                    None => None,
                }
            })
            .collect();

        AppicGetEventsResult {
            events: filtered_mapped,
            total_event_count: value.total_event_count,
        }
    }
}