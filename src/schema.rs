// @generated automatically by Diesel CLI.

diesel::table! {
    collections (id) {
        id -> Integer,
        path -> Text,
    }
}

diesel::table! {
    config (volume) {
        volume -> Float,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    collections,
    config,
);
