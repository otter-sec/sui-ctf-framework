module challenge::interactive_ctf {
    // ---------------------------------------------------
    // DEPENDENCIES
    // ---------------------------------------------------

    use sui::event;
    use sui::transfer;
    use sui::object::{Self, UID};
    use sui::tx_context::{Self, TxContext};
    use std::vector;

    // ---------------------------------------------------
    // STRUCTS
    // ---------------------------------------------------

    public struct Challenge has key, store {
        id: UID,
        counter: u64,
        secret: u64,
        solved: bool
    }

    public struct UserProgress has key, store {
        id: UID,
        user: address,
        steps_completed: u64,
        magic_number: u64
    }

    // ---------------------------------------------------
    // EVENTS
    // ---------------------------------------------------

    public struct StepCompleted has copy, drop {
        user: address,
        step: u64
    }

    public struct ChallengeAttempt has copy, drop {
        user: address,
        success: bool
    }

    // ---------------------------------------------------
    // CONSTANTS
    // ---------------------------------------------------

    const EINVALID_SOLUTION: u64 = 1337;
    const ESTEP_NOT_COMPLETED: u64 = 1338;
    const EWRONG_MAGIC: u64 = 1339;
    const SECRET_VALUE: u64 = 42;
    const REQUIRED_STEPS: u64 = 3;

    // ---------------------------------------------------
    // INIT FUNCTION
    // ---------------------------------------------------

    fun init(ctx: &mut TxContext) {
        let challenge = Challenge {
            id: object::new(ctx),
            counter: 0,
            secret: SECRET_VALUE,
            solved: false
        };
        transfer::share_object(challenge);
    }

    // ---------------------------------------------------
    // PUBLIC FUNCTIONS
    // ---------------------------------------------------

    public entry fun create_progress(ctx: &mut TxContext) {
        let progress = UserProgress {
            id: object::new(ctx),
            user: tx_context::sender(ctx),
            steps_completed: 0,
            magic_number: 0
        };
        transfer::transfer(progress, tx_context::sender(ctx));
    }

    public entry fun step_one(progress: &mut UserProgress, value: u64) {
        if (value == 100) {
            progress.steps_completed = 1;
            progress.magic_number = value * 2; // 200
            event::emit(StepCompleted {
                user: progress.user,
                step: 1
            });
        }
    }

    public entry fun step_two(progress: &mut UserProgress, challenge: &mut Challenge, list: vector<u8>) {
        assert!(progress.steps_completed >= 1, ESTEP_NOT_COMPLETED);
        
        // Check if list has correct values [1, 3, 3, 7]
        if (vector::length(&list) == 4 &&
            *vector::borrow(&list, 0) == 1 &&
            *vector::borrow(&list, 1) == 3 &&
            *vector::borrow(&list, 2) == 3 &&
            *vector::borrow(&list, 3) == 7) {
            
            progress.steps_completed = 2;
            challenge.counter = challenge.counter + 1;
            progress.magic_number = progress.magic_number + 137; // Now 337
            
            event::emit(StepCompleted {
                user: progress.user,
                step: 2
            });
        }
    }

    public entry fun step_three(progress: &mut UserProgress, challenge: &Challenge, guess: u64) {
        assert!(progress.steps_completed >= 2, ESTEP_NOT_COMPLETED);
        
        // User needs to guess the secret value
        if (guess == challenge.secret) {
            progress.steps_completed = 3;
            progress.magic_number = progress.magic_number + challenge.secret; // Now 379
            
            event::emit(StepCompleted {
                user: progress.user,
                step: 3
            });
        }
    }

    public entry fun final_solve(progress: &mut UserProgress, challenge: &mut Challenge) {
        assert!(progress.steps_completed >= REQUIRED_STEPS, ESTEP_NOT_COMPLETED);
        assert!(progress.magic_number == 379, EWRONG_MAGIC);
        
        challenge.solved = true;
        
        event::emit(ChallengeAttempt {
            user: progress.user,
            success: true
        });
    }

    // ---------------------------------------------------
    // CHECK SOLUTION
    // ---------------------------------------------------

    public entry fun check_solution(challenge: &Challenge) {
        assert!(challenge.solved == true, EINVALID_SOLUTION);
    }

    // ---------------------------------------------------
    // VIEW FUNCTIONS
    // ---------------------------------------------------

    public fun get_counter(challenge: &Challenge): u64 {
        challenge.counter
    }

    public fun get_progress(progress: &UserProgress): (u64, u64) {
        (progress.steps_completed, progress.magic_number)
    }

    public fun is_solved(challenge: &Challenge): bool {
        challenge.solved
    }
}