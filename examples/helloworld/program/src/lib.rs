use arch_program::{
    account::AccountInfo,
    bitcoin::{self, absolute::LockTime, transaction::Version, Transaction},
    entrypoint,
    helper::add_state_transition,
    input_to_sign::InputToSign,
    msg,
    program::{
        get_account_script_pubkey, get_bitcoin_block_height, next_account_info,
        set_transaction_to_sign,
    },
    program_error::ProgramError,
    pubkey::Pubkey,
    transaction_to_sign::TransactionToSign,
};
use borsh::{BorshDeserialize, BorshSerialize};

// Register our program's entrypoint function
entrypoint!(process_instruction);

/// Main program entrypoint. This function is called whenever someone wants
/// to interact with our Hello World program.
///
/// # Arguments
/// * `_program_id` - The public key of our program
/// * `accounts` - Array of accounts that this instruction will operate on
/// * `instruction_data` - The data passed to this instruction, containing the name
pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // We expect exactly one account for this program
    if accounts.len() != 1 {
        return Err(ProgramError::Custom(501));
    }

    // Get the current Bitcoin block height for reference
    let bitcoin_block_height = get_bitcoin_block_height();
    msg!("bitcoin_block_height {:?}", bitcoin_block_height);

    // Get an iterator over the accounts and get the first (and only) account
    let account_iter = &mut accounts.iter();
    let account = next_account_info(account_iter)?;

    msg!("account {:?}", account);

    // Deserialize the instruction data into our params struct
    let params: HelloWorldParams = borsh::from_slice(instruction_data).unwrap();

    // Deserialize the Bitcoin transaction that will be used for fees
    let fees_tx: Transaction = bitcoin::consensus::deserialize(&params.tx_hex).unwrap();

    // Create our greeting message
    let new_data = format!("Hello {}", params.name);

    // Check if we need to resize the account to fit our greeting
    let data_len = account.data.try_borrow().unwrap().len();
    if new_data.as_bytes().len() > data_len {
        account.realloc(new_data.len(), true)?;
    }

    // Get the script pubkey for this account
    let script_pubkey = get_account_script_pubkey(account.key);
    msg!("script_pubkey {:?}", script_pubkey);

    // Store our greeting in the account's data
    account
        .data
        .try_borrow_mut()
        .unwrap()
        .copy_from_slice(new_data.as_bytes());

    // Create a new Bitcoin transaction for our state transition
    let mut tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![],
        output: vec![],
    };

    // Add the state transition and fee information
    add_state_transition(&mut tx, account);
    tx.input.push(fees_tx.input[0].clone());

    // Create the transaction signing request
    let tx_to_sign = TransactionToSign {
        tx_bytes: &bitcoin::consensus::serialize(&tx),
        inputs_to_sign: &[InputToSign {
            index: 0,
            signer: account.key.clone(),
        }],
    };

    msg!("tx_to_sign{:?}", tx_to_sign);

    // Submit the transaction for signing
    set_transaction_to_sign(accounts, tx_to_sign)
}

/// Parameters passed to our program
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct HelloWorldParams {
    /// The name to say hello to
    pub name: String,
    /// Raw Bitcoin transaction for fees
    pub tx_hex: Vec<u8>,
}
