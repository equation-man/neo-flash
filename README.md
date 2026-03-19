# NEO FLASH  
Flash loan program for testing the Mega AMM stableswap protocol.  

## System Engineering Overview.  
This flash loan system follows atomic sandwich structure. Where the intended DeFi operations like MEV logic is sandwitched between receiving the loan and paying back the lender after the intended operation is completed.  
This flash loan system also acts as the lender for effective and properly controlled test and liquidity provision for testing the system.  

### Borrow.  
The program starts by borrowing tokens from the protocol's vault instead of an external lender, and transfers them to the borrowers account. A scratch PDA for loan is also created that stored some required details of this loan.   

### Swap/activity.  
This is where the MEV logic lives. Here, the system simulates the swap activity using the spl_token::transfer(). It is from here that the flash loan system interracts with an AMM.   

### Repay.  
The program performs validation including fees before ending the process.   

## Atomic design.  
All above occure quickly under one transaction atomically.
