create table songs_dg_tmp
(
    id            integer
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
    year          integer,
    genre         TEXT,
    track_number  integer,
    album_volume  integer,
    album_artist  TEXT,
    duration      sqlite_uint64 not null,
    lyrics        text
);

insert into songs_dg_tmp(id, path, collection_id, title, artist, album, year, genre, track_number, album_artist,
                         duration)
select id,
       path,
       collection_id,
       title,
       artist,
       album,
       datetime,
       genre,
       track_number,
       album_artist,
       duration
from songs;

drop table songs;

alter table songs_dg_tmp
    rename to songs;

create index songs_artist_index
    on songs (artist);

create index songs_collection_id_index
    on songs (collection_id);

create unique index songs_path_uindex
    on songs (path);
