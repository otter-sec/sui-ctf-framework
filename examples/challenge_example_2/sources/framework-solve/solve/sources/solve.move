module solution::solution {
    // [*] Import dependencies
    use sui::tx_context::TxContext;
    use std::vector;
    // use std::debug;
    use std::bcs;
    use challenge::MileHighCity;

    public entry fun solve(status: &mut MileHighCity::Status, ctx: &mut TxContext) {
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

        let sender_addr : address = @0x0;
        let sender_addr_bytes : vector<u8> = bcs::to_bytes(&sender_addr);

        let ciphertext : vector<u8> = vector::empty<u8>();
        let i = 0;

        while( i < vector::length(&original_plaintext) ) {
            let tmp1 : &u8 = vector::borrow(&original_plaintext, i);
            let tmp2 : &u8 = vector::borrow(&sender_addr_bytes, (i % 20));
            vector::push_back(&mut ciphertext, *tmp1 ^ *tmp2);

            i = i+1;
        };

        MileHighCity::travel(status, ciphertext, ctx);
    }
}
