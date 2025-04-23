# "Zero to Production in Rust" book walkthrough

This repository contains the code for the book [Zero to Production in Rust](https://www.zero2prod.com/) by Luca Palmieri.

The original source code is available on [Luca's repository](https://github.com/LukeMathWalker/zero-to-production). 

In this repository I'm just writing the code as I read through the book and follow it.

It's not intended as a replacement to the author original source code, just a place to experiment (and to run the pipelines explained in the book 😎). 

Please refer to the book and the original repository if you are looking for the correct examples!

Note: while the author uses digital ocean for the deploy, I used a small k3s cluster in my home lab.

### TODO (after reading the book)

Exercises left to the reader (searched through the book)
- [ ] Implemnt idempotency key expiration
- [ ] Installation procedure/seeding (e.g. for admin username/password)
- [ ] Add password validation in register/change password
- [ ] Retry and backoff for email delivery
