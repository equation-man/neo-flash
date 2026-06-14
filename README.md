# NEO FLASH FLUID SIPHON: MODELLING THE DYNAMICS OF A FLASH LOAN SYSTEM
A siphon uses gravity and pressure differentials to pull fluid out of a high reservoir, pass it through an intermediate system and perform mechanical work, and return it to a low reservoir, all driven by continous weight of the liquid column. If the siphon column breaks, the flow instantly stops and the system reverts to equilibrium. A flash loan works exactly like this continous fluid.  

It temporarily gives access to liquidity to a borrower who can then execute MEV strategy, and if anything breaks along the way, the transaction is reverted back to its initial state. In this analogy, the flash loan technically "siphon's" liquidity from the liquidity pool or a lending protocol to give it to a borrower. 

## Analogies:  
- Pool liquidity: The potential energy stored in the water reservoir.
- Flash loan: Siphoning water out instantly via high pressure drop.  
- Mechanical work: Could be arbitrage or any other MEV strategy.
- Single transaction atomicity: If transaction fails, everything is rolled back, same to when the fluid siphon break before the loop ends, the fluid state reverts.

## System Engineering Overview.  
This flash loan system follows atomic sandwich structure. Where the intended DeFi operations like MEV logic is sandwitched between receiving the loan and paying back the lender after the intended operation is completed.  
This flash loan system acts as the lender for effective and properly controlled test and liquidity provision for testing the system.  
#### Architecture.  
1. Borrow -> Verify repay exists -> Issue temporary liquidity.
2. Execute MEV strategy.
3. Repay -> Read borrow instruction -> Verify balances -> Repay back the loan.

### Initialize instruction.  
Initializes the protocol, setting up the authority, fee and other protocol configurations.  

### Borrow instruction.  
Issues temporary liquidity to the borrower. This is the flash loan.  

### Repay.  
Repays back the loan, with fee(profit).  

## Atomic design.  
All above occure quickly under one transaction atomically.
