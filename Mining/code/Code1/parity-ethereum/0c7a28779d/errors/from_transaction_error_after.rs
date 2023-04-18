pub fn from_transaction_error(error: EthcoreError) -> Error {
	use ethcore::error::TransactionError::*;

	if let EthcoreError::Transaction(e) = error {
		let msg = match e {
			AlreadyImported => "Transaction with the same hash was already imported.".into(),
			Old => "Transaction nonce is too low. Try incrementing the nonce.".into(),
			TooCheapToReplace => {
				"Transaction gas price is too low. There is another transaction with same nonce in the queue. Try increasing the gas price or incrementing the nonce.".into()
			},
			LimitReached => {
				"There are too many transactions in the queue. Your transaction was dropped due to limit. Try increasing the fee.".into()
			},
			InsufficientGasPrice { minimal, got } => {
				format!("Transaction gas price is too low. It does not satisfy your node's minimal gas price (minimal: {}, got: {}). Try increasing the gas price.", minimal, got)
			},
			InsufficientBalance { balance, cost } => {
				format!("Insufficient funds. Account you try to send transaction from does not have enough funds. Required {} and got: {}.", cost, balance)
			},
			GasLimitExceeded { limit, got } => {
				format!("Transaction cost exceeds current gas limit. Limit: {}, got: {}. Try decreasing supplied gas.", limit, got)
			},
			InvalidGasLimit(_) => "Supplied gas is beyond limit.".into(),
		};
		Error {
			code: ErrorCode::ServerError(codes::TRANSACTION_ERROR),
			message: msg,
			data: None,
		}
	} else {
		Error {
			code: ErrorCode::ServerError(codes::UNKNOWN_ERROR),
			message: "Unknown error when sending transaction.".into(),
			data: Some(Value::String(format!("{:?}", error))),
		}
	}
}
