module challenge::MileHighCity {
    
    // [*] Import dependencies
    use std::vector;
    use std::bcs;
    use sui::transfer;
    use sui::object::{Self, UID};
    use sui::tx_context::{Self, TxContext};
 
    // [*] Structs
    struct Status has key, store {
        id : UID,
        solved : bool,
    }

    // [*] Module initializer
    fun init(ctx: &mut TxContext) {

        transfer::share_object(Status {
            id: object::new(ctx),
            solved: false
        })

    }

    // [*] Public functions
    public entry fun travel(status: &mut Status, ciphertext : vector<u8>, ctx: &mut TxContext) {
        
        let original_plaintext : vector<u8> = vector[73,110,115,116,101,97,100,32,111,102,32,
                                         112,117,116,116,105,110,103,32,116,104,
                                         101,32,116,97,120,105,32,100,114,105,118,
                                         101,114,32,111,117,116,32,111,102,32,97,
                                         32,106,111,98,44,32,98,108,111,99,107,99,
                                         104,97,105,110,32,112,117,116,115,32,85,
                                         98,101,114,32,111,117,116,32,111,102,32,
                                         97,32,106,111,98,32,97,110,100,32,108,101,
                                         116,115,32,116,104,101,32,116,97,120,105,
                                         32,100,114,105,118,101,114,115,32,119,111,
                                         114,107,32,119,105,116,104,32,116,104,101,
                                         32,99,117,115,116,111,109,101,114,32,100,
                                         105,114,101,99,116,108,121,46];

        let sender_addr : address = tx_context::sender(ctx);
        let sender_addr_bytes : vector<u8> = bcs::to_bytes(&sender_addr);

        let plaintext : vector<u8> = vector::empty<u8>();
        let i = 0;

        while( i < vector::length(&ciphertext) ) {
            let tmp1 : &u8 = vector::borrow(&ciphertext, i);
            let tmp2 : &u8 = vector::borrow(&sender_addr_bytes, (i % 20));
            vector::push_back(&mut plaintext, *tmp1 ^ *tmp2);

            i = i+1;
        };

        assert!(plaintext == original_plaintext, 0);

        status.solved = true;
    }

    public entry fun check_status(status: &mut Status) {
        assert!(status.solved == true, 0);
    }
}
