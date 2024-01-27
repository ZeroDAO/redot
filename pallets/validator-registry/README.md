# Validator Registry Pallet

This pallet provides functionality for managing a registry of validators.
It allows for the registration and updating of validator information, handling both on-chain and off-chain data.
The pallet supports off-chain workers for data processing and employs unsigned transactions with custom validation logic.

## Overview

- **Validator Registration**: Validators can register themselves using an on-chain transaction.
- **Validator Update**: Validator information can be updated, including adding and removing validators.
- **Off-chain Workers**: Employed for processing data off-chain, contributing to scalability.
- **Unsigned Transactions**: Utilized for off-chain communication, with custom validation logic to ensure integrity.