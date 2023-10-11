-- noinspection SqlWithoutWhere
delete
from bodies;

alter table bodies
    drop column navigation_type;
