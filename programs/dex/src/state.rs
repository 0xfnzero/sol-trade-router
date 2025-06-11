use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct TradeFeeState {
    pub fee_rate: u8,
    pub fee_wallet: Pubkey,
}