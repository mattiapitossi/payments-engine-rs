# payments-engine-rs
An implementation of a payment engine to process transactions and update client accounts. See [architecture docs](docs/ARCHITECTURE.md) for additional design info.


## Features 
- CLI powered by clap to offer a intuitive interface
- Logging without exposing sensitive information
- Input validation to ensure consistency across accounts

## Requirements 

The input CSV should have 4 columns:
- type: which can be `deposit`, `withdrawal`, `dispute`, `resolve`, `chargeback`
- client: the unique id of the client
- tx: the unique id of the transaction
- amount: decimal value with up to 4 decimal places, required only for deposit and withdrawal. 

Validations:

- blocking:
  - the type should be one of the supported ones with lowercase format, if the CSV contains a different transaction type the process will fail
  - the transaction id should be unique, if two or more transactions (related to deposit or withdrawal) have the same id the process will fail
  - the amount is mandatory for deposit or withdrawal, if it's missing, has a negative value, or the scale of the amount is greater than 4 the process will fail
- non-blocking:
  - if a client request to withdraw more money than available the withdrawal fails, but the processing continues
  - if a dispute, a resolution or chargeback transaction refers to a non-existing transaction the request will be ignored
  - if a dispute, a resolution or chargeback transaction refers to an existing transaction but wrong client the request will be ignored
  - if a resolution or a chargeback transaction refers to an existing transaction and right client, but the transaction is not under dispute the request will be ignored
  - if a dispute, a resolution, or a chargeback contains an amount, the value will be ignored
  - if a locked account performs additional operations, requests will be ignored

if any blocking error occurs, the validation message goes to stderr. This is to separate errors from stdout.

- example err when prompting a wrong path (e.g., a path named `ex` that does not exist): `Error: cannot find path ex`
- example err when transaction ids are not unique: `Error: Transaction ids are not unique!`


## Examples

The project includes an examples folder that can be used to run the program:

For example, a chargeback can be run as follows:

`cargo run -- examples/chargeback.csv > out.csv`

## Roadmap
- DB layer for persistence
- REST API to communicate via network

## Develop

A Justfile is provided for simplify the development, run `just check` for checking format and a security audit of deps
