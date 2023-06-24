-- noinspection SqlAddNotNullColumnForFile

alter table songs
    add title TEXT not null;

alter table songs
    add artist TEXT not null;

alter table songs
    add album TEXT not null;

alter table songs
    add datetime sqlite_uint64;

alter table songs
    add genre TEXT not null;

alter table songs
    add track_number integer not null;

alter table songs
    add album_artist TEXT;
