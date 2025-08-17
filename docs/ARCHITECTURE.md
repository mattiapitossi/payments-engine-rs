# Architecture

This document is a high-level description of the architecture of the payment engine.

Payment engine is responsible for processing customer transaction requests. Each data batch (stored in a CSV file), contains different transactions that will be validated upfront. The first service validates the transaction batch by ensuring that each record follows the requirements; if any of the transaction fails the validation the whole request will be discarded.

The service responsible for processing the request is the engine, which handles each type of transaction and acts as a dispatcher that process each request. The engine is also a robust system, that checks if a transaction dispute, resolution, or chargeback belongs to a specific customer before processing the request.


## Technical Features

- CLI: to offer a comprehensive set of features, clap was used to offer the help and usage functionalities.
- Logs: logs are also available to help the developers in debugging errors. Notice that internal errors that are not related to wrong inputs are not exposed to the client.
- Errors: Anyhow is used to simplify the errors propagation. Errors related to a wrong input are written on stdout.
- Domain entities validate against wrong usage of the methods, from a engine perspective we know the transaction type, but if used outside we should be careful to not accept wrong operations.


## Domain Entities

- `CashFlow`: a cash flow entity represents either a deposit or a withdrawal transaction, has an under dispute attribute to track the status of a dispute filled by the customer. The amount is not optional in this case. Storing deposit and withdrawal transactions into cash flow entity also allows to perform and upfront validation and subsequently make safe assumptions about these two entities.
- `Account`: an account represents the latest snapshot of the customer holdings.

## Future Work

A persistence layer could be added to store the domain entity data, the dispute and the subsequent resolution or the chargeback could also be stored and represented by a different domain entity to track and let the cash flow be responsible of tracking only a transaction where the amount is present and if it's under dispute. 

The DB layer will be also useful to process different batches, for example a first batch could indicate that a transaction was charged back, but a second batch could try to perform additional operations related to a locked account.

The engine could also add support for concurrent processing, in this case we should be mindful of concurrent access to the resources and to end up with a consistent result.
