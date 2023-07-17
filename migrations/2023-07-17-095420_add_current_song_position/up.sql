alter table config
    add current_song_position sqlite_uint64 default 0 not null;
