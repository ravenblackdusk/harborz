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
        current_song_id -> Nullable<Integer>,
        window_width -> Integer,
        window_height -> Integer,
        maximized -> Integer,
    }
}

diesel::table! {
    songs (id) {
        id -> Integer,
        path -> Text,
        collection_id -> Integer,
        title -> Nullable<Text>,
        artist -> Nullable<Text>,
        album -> Nullable<Text>,
        datetime -> Nullable<BigInt>,
        genre -> Nullable<Text>,
        track_number -> Nullable<Integer>,
        album_artist -> Nullable<Text>,
        duration -> BigInt,
    }
}

diesel::joinable!(config -> songs (current_song_id));
diesel::joinable!(songs -> collections (collection_id));

diesel::allow_tables_to_appear_in_same_query!(
    collections,
    config,
    songs,
);
