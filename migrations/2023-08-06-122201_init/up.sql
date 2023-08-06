create table bodies
(
    id                integer not null
        constraint bodies_pk
            primary key autoincrement,
    query1            TEXT,
    body_type         TEXT    not null,
    scroll_adjustment REAL,
    navigation_type   TEXT    not null,
    query2            TEXT
);

create table collections
(
    id       integer not null
        constraint collections_pk
            primary key autoincrement,
    path     TEXT    not null,
    modified sqlite_uint64
);

create unique index collections_path_uindex
    on collections (path);

create table songs
(
    id            integer       not null
        constraint songs_pk
            primary key autoincrement,
    path          TEXT          not null,
    collection_id integer       not null
        constraint songs_collections_id_fk
            references collections
            on update cascade on delete cascade,
    title         TEXT,
    artist        TEXT,
    album         TEXT,
    datetime      sqlite_uint64,
    genre         TEXT,
    track_number  integer,
    album_artist  TEXT,
    duration      sqlite_uint64 not null
);

create table config
(
    current_song_position     sqlite_uint64     not null
        constraint config_pk
            primary key,
    current_song_id           integer
        constraint config_songs_id_fk
            references songs
            on update cascade on delete set null,
    window_width              integer default 0 not null,
    window_height             integer default 0 not null,
    maximized                 integer default 0 not null,
    now_playing_body_realized integer default 0 not null
);

insert into config(current_song_position)
VALUES (0);

create index songs_artist_index
    on songs (artist);

create index songs_collection_id_index
    on songs (collection_id);

create unique index songs_path_uindex
    on songs (path);
