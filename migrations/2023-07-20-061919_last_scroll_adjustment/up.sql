create table config_dg_tmp
(
    volume                REAL                    not null,
    current_song_id       integer
        constraint config_pk
            primary key
        constraint config_songs_id_fk
            references songs
            on update cascade on delete set null,
    window_width          integer       default 0 not null,
    window_height         integer       default 0 not null,
    maximized             integer       default 0 not null,
    current_song_position sqlite_uint64 default 0 not null
);

insert into config_dg_tmp(volume, current_song_id, window_width, window_height, maximized, current_song_position)
select volume, current_song_id, window_width, window_height, maximized, current_song_position
from config;

drop table config;

alter table config_dg_tmp
    rename to config;

alter table config
    drop column volume;

alter table history_bodies
    add scroll_adjustment REAL;
