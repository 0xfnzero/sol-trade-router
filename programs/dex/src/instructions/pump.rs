use solana_program::{
    account_info::{next_account_info, AccountInfo},
    borsh::try_from_slice_unchecked,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    pubkey,
};

use crate::state::TradeFeeState;

const PUMPFUN_BUY_SELECTOR: &[u8; 8] = &[102, 6, 61, 18, 1, 218, 235, 234];
const PUMPFUN_SELL_SELECTOR: &[u8; 8] = &[51, 230, 133, 164, 1, 127, 131, 173];
const PUMPAMM_BUY_SELECTOR: &[u8; 8] = &[102, 6, 61, 18, 1, 218, 235, 234];
const PUMPAMM_SELL_SELECTOR: &[u8; 8] = &[51, 230, 133, 164, 1, 127, 131, 173];

pub const PUMP_SELECTOR: &[u8; 8] = &[82, 225, 119, 231, 78, 29, 45, 70];
pub const PUMP_AMM_SELECTOR: &[u8; 8] = &[129, 59, 179, 195, 110, 135, 61, 2];
pub const PUMP_SELL_SELECTOR: &[u8; 8] = &[83, 225, 119, 231, 78, 29, 45, 70];
pub const PUMP_AMM_SELL_SELECTOR: &[u8; 8] = &[130, 59, 179, 195, 110, 135, 61, 2];

const PUMP_PROGRAM: Pubkey = pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
const PUMP_AMM_PROGRAM_ID: Pubkey = pubkey!("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA");

const ARG_LEN: usize = 24;

fn to_account_metas(accounts: &[AccountInfo]) -> Vec<AccountMeta> {
    let mut metas = Vec::with_capacity(accounts.len());
    metas.append(
        &mut accounts
            .iter()
            .map(|acc| match acc.is_writable {
                false => AccountMeta::new_readonly(*acc.key, acc.is_signer),
                true => AccountMeta::new(*acc.key, acc.is_signer),
            })
            .collect(),
    );
    metas
}

fn calculate_fee(amount: u64, fee_rate: u8) -> u64 {
    (amount as f64 * fee_rate as f64 / 100.0) as u64
}

fn process_with_fee(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
    selector: &[u8; 8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    
    // 安全获取账户
    let fee_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    let fee_payer = next_account_info(accounts_iter)?; // 支付手续费的SOL账户
    let fee_receiver = next_account_info(accounts_iter)?; // 接收手续费的SOL账户
    
    // 验证支付者账户签名
    if !fee_payer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    // 反序列化配置
    let mut trade_fee_config = TradeFeeState::try_from_slice(&fee_account.data.borrow())?;
    
    // 验证接收方地址匹配配置
    if fee_receiver.key != &trade_fee_config.fee_wallet {
        return Err(ProgramError::InvalidAccountData);
    }
    
    // 解析金额
    if instruction_data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let amount = u64::from_le_bytes(
        instruction_data[0..8]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    );
    
    // 计算费用
    let fee = (amount * trade_fee_config.fee_rate as u64) / 100;
    let remaining_amount = amount.checked_sub(fee)
        .ok_or(ProgramError::InsufficientFunds)?;
    
    // 验证支付者有足够余额
    if **fee_payer.lamports.borrow() < fee {
        return Err(ProgramError::InsufficientFunds);
    }
    
    // 转账SOL手续费到协议钱包
    invoke(
        &system_instruction::transfer(
            fee_payer.key,
            fee_receiver.key,
            fee,
        ),
        &[
            fee_payer.clone(),
            fee_receiver.clone(),
            system_program.clone(),
        ],
    )?;
    
    // 构建原始指令数据（保持原始数据不变）
    let mut data = Vec::with_capacity(8 + instruction_data.len() - 8);
    data.extend_from_slice(selector);
    data.extend_from_slice(&instruction_data[8..]);
    
    // 更新金额为扣除费用后的剩余金额
    data[8..16].copy_from_slice(&remaining_amount.to_le_bytes());
    
    // 执行原始交易（使用剩余账户）
    invoke(
        &Instruction {
            program_id: *program_id,
            accounts: accounts[4..] // 跳过已处理的账户
                .iter()
                .map(|acc| AccountMeta {
                    pubkey: *acc.key,
                    is_signer: acc.is_signer,
                    is_writable: acc.is_writable,
                })
                .collect(),
            data,
        },
        &accounts[4..],
    )
}

pub fn process_pump_buy(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
    process_with_fee(&PUMP_PROGRAM, accounts, instruction_data, PUMPFUN_BUY_SELECTOR)
}

pub fn process_pump_amm_buy(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
    process_with_fee(&PUMP_AMM_PROGRAM_ID, accounts, instruction_data, PUMPAMM_BUY_SELECTOR)
}

pub fn process_pump_sell(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
    process_with_fee(&PUMP_PROGRAM, accounts, instruction_data, PUMPFUN_SELL_SELECTOR)
}

pub fn process_pump_amm_sell(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
    process_with_fee(&PUMP_AMM_PROGRAM_ID, accounts, instruction_data, PUMPAMM_SELL_SELECTOR)
}
