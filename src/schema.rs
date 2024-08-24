// @generated automatically by Diesel CLI.

diesel::table! {
    invoice (address) {
        #[max_length = 42]
        address -> Bpchar,
        #[max_length = 42]
        receiver -> Bpchar,
        mnemonic -> Varchar,
        state -> Int4,
        value -> Float8,
        lifetime -> Int4,
        complete_action -> Int4,
    }
}
