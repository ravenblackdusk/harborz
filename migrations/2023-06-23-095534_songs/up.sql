create table songs
(
    id            integer not null
        constraint songs_pk
            primary key autoincrement,
    path          TEXT    not null,
    collection_id integer not null
        constraint songs_collections_id_fk
            references collections
            on update cascade on delete cascade
);

-- noinspection SpellCheckingInspection @ index/"songs_path_uindex"
create unique index songs_path_uindex
    on songs (path);

alter table collections
    add modified sqlite_uint64;
