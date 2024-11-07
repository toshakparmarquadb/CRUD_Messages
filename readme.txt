// Project Runable Commands..

1. Create Messages: 
    dfx canister call crud_messages_backend create_message '("Hello World!", null)'

2. Show & Read Messages: 
    dfx canister call crud_messages_backend get_message '(1 : nat64)'

3. Show & Read All Messages: (Not Working Yet).
    dfx canister call crud_messages_backend get_messages '(record { page = 1; limit = 10; sort_by = opt "newest" })'

4. Update Messages:
    dfx canister call crud_messages_backend update_message '(1 : nat64, "Hello Area")'

5. Delete Messages: 
    dfx canister call crud_messages_backend delete_message '(1 : nat64)'



// Some Important Dependencies
crud_messages_backend canister id : b77ix-eeaaa-aaaaa-qaada-cai

[package]
name = "crud_messages_backend"
version = "0.1.0"
edition = "2021"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[lib]
crate-type = ["cdylib"]

[dependencies]
candid = "0.10"
ic-cdk = "0.16"
ic-cdk-timers = "0.10" # Feel free to remove this dependency if you don't need timers