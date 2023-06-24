// @generated automatically by Diesel CLI.

diesel::table! {
    collections (id) {
        id -> Integer,
        path -> Text,
        row -> Integer,
        modified -> Nullable<BigInt>,
    }
}

diesel::table! {
    config (volume) {
        volume -> Float,
    }
}

diesel::table! {
    songs (id) {
        id -> Integer,
        path -> Text,
        collection_id -> Integer,
    }
}

diesel::joinable!(songs -> collections (collection_id));

diesel::allow_tables_to_appear_in_same_query!(
    collections,
    config,
    songs,
);
