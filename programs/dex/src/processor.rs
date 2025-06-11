use solana_program::{
    account_info::AccountInfo, 
    entrypoint::ProgramResult, 
    program_error::ProgramError,
    pubkey::Pubkey,
};

use borsh::{BorshDeserialize,BorshSerialize};

use crate::instructions::ata::{process_create_associated_token_account, ATA_SELECTOR};
use crate::instructions::pump::{
    process_pump_amm_buy, process_pump_amm_sell, process_pump_buy, process_pump_sell,
    PUMP_AMM_SELL_SELECTOR, PUMP_AMM_SELECTOR, PUMP_SELL_SELECTOR, PUMP_SELECTOR,
};
use crate::instructions::raydium::{process_raydium_buy, process_raydium_sell, RAYDIUM_BUY_SELECTOR, RAYDIUM_SELL_SELECTOR};
use crate::instructions::slot::{process_expired_slot, EXPIRED_SLOT_SELECTOR};
use crate::state::TradeFeeState;

type SelectorHandler = fn(&[AccountInfo], &[u8]) -> ProgramResult;

// 添加设置协议费钱包的选择器
const SET_PROTOCOL_FEE_WALLET_SELECTOR: &[u8; 8] = b"set_fee\0";

const SELECTORS: [(&[u8; 8], SelectorHandler); 9] = [  // 注意数组大小改为9
    (PUMP_SELECTOR, |accounts, rest| {
        process_pump_buy(accounts, rest)
    }),
    (PUMP_AMM_SELECTOR, |accounts, rest: &[u8]| {
        process_pump_amm_buy(accounts, rest)
    }),
    (PUMP_SELL_SELECTOR, |accounts, rest| {
        process_pump_sell(accounts, rest)
    }),
    (PUMP_AMM_SELL_SELECTOR, |accounts, rest| {
        process_pump_amm_sell(accounts, rest)
    }),
    (ATA_SELECTOR, |accounts, rest| {
        process_create_associated_token_account(accounts, rest)
    }),
    (EXPIRED_SLOT_SELECTOR, |_, rest| process_expired_slot(rest)),
    (RAYDIUM_BUY_SELECTOR, |accounts, rest| {
        process_raydium_buy(accounts, rest)
    }),
    (RAYDIUM_SELL_SELECTOR, |accounts, rest| {
        process_raydium_sell(accounts, rest)
    }),
    // 添加设置协议费钱包的路由
    (SET_PROTOCOL_FEE_WALLET_SELECTOR, |accounts, rest| {
        set_protocol_fee_wallet(accounts, rest)
    }),
];

pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let (method, rest) = instruction_data.split_at(8);

    for (selector, handler) in SELECTORS.iter() {
        if method == selector.as_slice() {
            return handler(accounts, rest);
        }
    }

    Err(ProgramError::InvalidInstructionData)
}

// 修复1：添加初始化配置账户函数
pub fn initialize_config_account(
    accounts: &[AccountInfo],
    fee_rate: u8,
) -> ProgramResult {
    let config_account = &accounts[0];
    let admin = &accounts[1];
    
    // 验证管理员签名
    if !admin.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    // 初始化配置
    let config = TradeFeeState {
        fee_rate,
        fee_wallet: *admin.key,  // 初始化为管理员地址
    };
    
    config.serialize(&mut &mut config_account.data.borrow_mut()[..])?;
    Ok(())
}

// 修复2：修改函数签名并添加权限检查
pub fn set_protocol_fee_wallet(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // 解析新钱包地址 (32字节)
    if instruction_data.len() < 32 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let new_wallet = Pubkey::new_from_array(<[u8; 32]>::try_from(&instruction_data[..32]).unwrap());

    // 账户验证
    let fee_account = &accounts[0];
    let admin_account = &accounts[1];
    
    // 1. 验证管理员签名
    if !admin_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    // 2. 反序列化配置
    let mut trade_fee_config = TradeFeeState::try_from_slice(&fee_account.data.borrow())?;
    
    // 3. 验证调用者是当前管理员
    if *admin_account.key != trade_fee_config.fee_wallet {
        return Err(ProgramError::IllegalOwner);
    }
    
    // 4. 更新协议费钱包地址
    trade_fee_config.fee_wallet = new_wallet;
    
    // 5. 序列化并存储
    trade_fee_config.serialize(&mut &mut fee_account.data.borrow_mut()[..])?;

    Ok(())
}