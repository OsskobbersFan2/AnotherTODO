create table task (
    id bigserial not null primary key,
    task_name varchar not null,
    task_description varchar,
    deadline date not null,
    task_status status not null
);
