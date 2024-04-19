module challenge::OtterLoan {
   
    // ---------------------------------------------------
    // DEPENDENCIES
    // ---------------------------------------------------   
   
    use sui::coin::{Self, Coin};
    use sui::balance::{Self, Balance};
    use sui::object::{Self, ID, UID};
    use sui::transfer;
    use sui::tx_context::{Self, TxContext};

    // ---------------------------------------------------
    // STRUCTS
    // ---------------------------------------------------

    public struct OSEC has drop {}

    public struct FlashLender<phantom T> has key {
        id: UID,
        to_lend: Balance<T>,
        fee: u64,
    }

    public struct Receipt<phantom T> has drop{
        flash_lender_id: ID,
        repay_amount: u64
    }

    public struct AdminCapability has key, store {
        id: UID,
        flash_lender_id: ID,
    }

    // ---------------------------------------------------
    // CONSTANTS
    // ---------------------------------------------------

    const ELOAN_TOO_LARGE: u64 = 0;
    const EINVALID_REPAYMENT_AMOUNT: u64 = 1;
    const EREPAY_TO_WRONG_LENDER: u64 = 2;
    const EADMIN_ONLY: u64 = 3;
    const EWITHDRAW_TOO_LARGE: u64 = 4;

    // ---------------------------------------------------
    // FUNCTIONS
    // ---------------------------------------------------

    public fun new<CoinType>( to_lend: Coin<CoinType>, fee: u64, ctx: &mut TxContext ): AdminCapability {

        let flash_lender_uid : UID = object::new(ctx);
        let flash_lender_id = object::uid_to_inner(&flash_lender_uid);
        let balance_to_lend: Balance<CoinType> = coin::into_balance(to_lend);
        let flash_lender = FlashLender { 
            id: flash_lender_uid, 
            to_lend: balance_to_lend, 
            fee: fee 
        };

        transfer::share_object(flash_lender);
        AdminCapability { id: object::new(ctx), flash_lender_id }
    }

    public fun create<CoinType>( to_lend: Coin<CoinType>, fee: u64, ctx: &mut TxContext ) {

        let admin_cap = new(to_lend, fee, ctx);
        transfer::transfer(admin_cap, tx_context::sender(ctx))
    }

  
    public fun loan<CoinOut>( self: &mut FlashLender<CoinOut>, amount: u64, ctx: &mut TxContext ): (Coin<CoinOut>, Receipt<CoinOut>) {

        let to_lend = &mut self.to_lend;
        assert!(balance::value(to_lend) >= amount, ELOAN_TOO_LARGE);
        let loan = coin::take(to_lend, amount, ctx);

        let repay_amount = amount + self.fee;        
        let receipt = Receipt { flash_lender_id: object::uid_to_inner(&self.id), repay_amount };
        (loan, receipt)
    }


    public fun repay<CoinIn>( self: &mut FlashLender<CoinIn>, payment: Coin<CoinIn>, receipt: Receipt<CoinIn> ) {

        let Receipt { flash_lender_id, repay_amount } = receipt;
        assert!(object::uid_to_inner(&self.id) == flash_lender_id, EREPAY_TO_WRONG_LENDER);
        assert!(coin::value(&payment) == repay_amount, EINVALID_REPAYMENT_AMOUNT);

        coin::put(&mut self.to_lend, payment)
    }

    public fun withdraw<CoinOut>( self: &mut FlashLender<CoinOut>, admin_cap: &AdminCapability, amount: u64, ctx: &mut TxContext ) {

        check_admin(self, admin_cap);

        let to_lend = &mut self.to_lend;
        assert!(balance::value(to_lend) >= amount, EWITHDRAW_TOO_LARGE);
        let coins = coin::take(to_lend, amount, ctx);

        let sender = tx_context::sender(ctx);
        transfer::public_transfer(coins, sender)
    }


    public fun deposit<CoinIn>( self: &mut FlashLender<CoinIn>, admin_cap: &AdminCapability, coin: Coin<CoinIn>, _ctx: &mut TxContext ) {
        
        check_admin(self, admin_cap);
        coin::put(&mut self.to_lend, coin)
    }

   
    public fun update_fee<CoinType>( self: &mut FlashLender<CoinType>, _admin_cap: &AdminCapability, new_fee: u64, _ctx: &mut TxContext ) {
        self.fee = new_fee
    }


    fun check_admin<CoinType>(self: &FlashLender<CoinType>, admin_cap: &AdminCapability ) {
        assert!(object::uid_to_inner(&self.id) == admin_cap.flash_lender_id, EADMIN_ONLY);
    }


    public fun fee<CoinType>( self: &FlashLender<CoinType> ): u64 {
        self.fee
    }

    
    public fun max_loan<CoinType>( self: &FlashLender<CoinType> ): u64 {
        balance::value(&self.to_lend)
    }

    
    public fun repay_amount<CoinType>( self: &Receipt<CoinType> ): u64 {
        self.repay_amount
    }

    
    public fun flash_lender_id<CoinType>(self: &Receipt<CoinType>): ID {
        self.flash_lender_id
    }

    // ---------------------------------------------------
    // TESTS
    // ---------------------------------------------------

    #[test_only]
    use sui::test_scenario::{Self, next_tx};

    #[test_only]
    use sui::coin::{mint_for_testing};

    #[test_only]
    use sui::sui::SUI;

    #[test]
    fun test_flashloan() {
        let lender = @0x111;
        let borrower = @0x222;
        let _contract = @admin;

        let mut scenario_val = test_scenario::begin(lender);
        let scenario = &mut scenario_val;

        next_tx(scenario, lender);
        {
            let coin : Coin<SUI> = mint_for_testing(500, test_scenario::ctx(scenario));
            create(coin, 100, test_scenario::ctx(scenario));
        };

        next_tx(scenario, borrower);
        {
            let mut flash_lender = test_scenario::take_shared<FlashLender<SUI>>(scenario);
            let (mut tokens, receipt) = loan(&mut flash_lender, 100, test_scenario::ctx(scenario));
            assert!(coin::value(&tokens) == 100, 0);
            let fee : Coin<SUI> = mint_for_testing(100, test_scenario::ctx(scenario));
            coin::join(&mut tokens, fee);
            repay(&mut flash_lender, tokens, receipt);
            test_scenario::return_shared(flash_lender);
        };

        test_scenario::end(scenario_val);
    }
}