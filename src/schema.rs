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
        title -> Text,
        artist -> Text,
        album -> Text,
        datetime -> Nullable<BigInt>,
        genre -> Text,
        track_number -> Integer,
        album_artist -> Nullable<Text>,
    }
}

diesel::joinable!(songs -> collections (collection_id));

diesel::allow_tables_to_appear_in_same_query!(
    collections,
    config,
    songs,
);
