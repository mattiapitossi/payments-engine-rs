# payments-engine-rs
An implementation of a payment engine to process transactions and update client accounts


## :rocket features 
- CLI powered by clap to offer a intuitive interface

## requirements 

The input CSV should have 4 columns:
- type: which can be `deposit`, `withdrawal`, `dispute`, `resolve`, `chargeback`
- client: the unique id of the client
- tx: the unique id of the transaction
- amount: decimal value with up to 4 decimal places, required only for deposit and withdrawal. 

Validations:
- the type should be one of the supported ones with lowercase format, if the CSV contains a different transaction the process will fail
- the transaction id should be unique, if two or more transactions have the same id the process will fail
- the amount is mandatory for deposit or withdrawal, if it's missing or has a negative value the process will fail

if any error occurs, the validation prints to stdout

## roadmap
- DB layer for persistence
- REST API to communicate via network
