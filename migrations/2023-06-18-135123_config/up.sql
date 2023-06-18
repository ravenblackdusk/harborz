create table config
(
    volume REAL not null
        constraint config_pk
            primary key
);

insert into config(volume)
values (0.5);
