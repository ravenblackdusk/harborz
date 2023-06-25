-- noinspection SqlAddNotNullColumnForFile

alter table songs
    add title TEXT;

alter table songs
    add artist TEXT;

alter table songs
    add album TEXT;

alter table songs
    add datetime sqlite_uint64;

alter table songs
    add genre TEXT;

alter table songs
    add track_number integer;

alter table songs
    add album_artist TEXT;
