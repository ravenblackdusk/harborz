alter table config
    add column current_song_id integer
        constraint config_songs_id_fk
            references songs
            on update cascade on delete set null;
