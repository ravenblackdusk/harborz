// @generated automatically by Diesel CLI.

diesel::table! {
    bodies (id) {
        id -> Integer,
        query1 -> Nullable<Text>,
        query2 -> Nullable<Text>,
        body_type -> crate::body::BodyTypeMapping,
        scroll_adjustment -> Nullable<Float>,
        navigation_type -> crate::body::NavigationTypeMapping,
    }
}

diesel::table! {
    collections (id) {
        id -> Integer,
        path -> Text,
        row -> Integer,
        modified -> Nullable<BigInt>,
    }
}

diesel::table! {
    config (current_song_id) {
        current_song_id -> Nullable<Integer>,
        window_width -> Integer,
        window_height -> Integer,
        maximized -> Integer,
        current_song_position -> BigInt,
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
    bodies,
    collections,
    config,
    songs,
);
