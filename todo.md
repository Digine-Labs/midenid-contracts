# Miden Name Registry - TODO

## Features

### Total Domain Counter
- [x] **Implementation**
  - [x] Add storage slot for domain counter in [masm/accounts/naming.masm](masm/accounts/naming.masm)
  - [x] Add internal procedure for incrementing
  - [x] Increment domains once new register
- [ ] **Tests**
  - [ ] Update naming contract init note with new storage length
  - [ ] Test domain counter works correctly

### Referral System
- [ ] **Implementation**
  - [x] Implement `set_referral_rate` function
  - [ ] Implement referral earning withdraws 
  - [ ] Referred register in register function or seperate function
  - [ ] Accounting for ref earnings and seperate it with protocol treasury earnings
- [ ] **Tests**
  - [] Test `set_referral_rate` function

### Pricing Contract Removal
- [ ] **Implementation**
  - [ ] Implement pricing features into naming contract
  - [ ] Seperate functions into different file than naming.masm Import letter count utils
- [ ] **Tests**
  - [ ] Test `set_referral_rate` function

---

## Completed Tasks
<!-- Move completed items here for reference -->

---