# Architecture

This document is a high-level description of the architecture of the payment engine.

Payment engine is responsible for processing customers transactions requests. Each data batch (stored in a CSV file), contains different transactions that will be validated upfront. The first service validates the transaction batch by ensuring that each record follows the requirements; if any of the transaction fails the validation the whole request will be discarded.


## Technical Features

- CLI: 

## Domain Entities

- `CashFlow`: a cash flow entity represents either a deposit or a withdrawal transaction, has an under dispute attribute to track the status of a dispute filled by the customer. The amount is not optional in this case.
- `Account`: an account represents the latest snapshot of the customer holdings.

## Future Work

A persistence layer could be added to store the domain entity data, the dispute and the subsequent resolution or the chargeback could also be stored and represented by a different domain entity to track and let the cash flow be responsible of tracking only a transaction where the amount is present.

