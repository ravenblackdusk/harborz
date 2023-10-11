// @generated automatically by Diesel CLI.

diesel::table! {
    bodies (id) {
        id -> Integer,
        body_type -> crate::body::BodyTypeMapping,
        scroll_adjustment -> Nullable<Float>,
        params -> Text,
    }
}

diesel::table! {
    collections (id) {
        id -> Integer,
        path -> Text,
        modified -> Nullable<BigInt>,
    }
}

diesel::table! {
    config (current_song_position) {
        current_song_position -> BigInt,
        current_song_id -> Nullable<Integer>,
        window_width -> Integer,
        window_height -> Integer,
        maximized -> Integer,
        now_playing_body_realized -> Integer,
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
        year -> Nullable<Integer>,
        genre -> Nullable<Text>,
        track_number -> Nullable<Integer>,
        album_volume -> Nullable<Integer>,
        album_artist -> Nullable<Text>,
        duration -> BigInt,
        lyrics -> Nullable<Text>,
    }
}

diesel::joinable!(config -> songs (current_song_id));
diesel::joinable!(songs -> collections (collection_id));

diesel::allow_tables_to_appear_in_same_query!(
    bodies,
    collections,
    config,
    songs,
);
