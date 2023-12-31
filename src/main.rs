use chrono::NaiveDate;
use comfy_table::{
    modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, Color, ContentArrangement, Table,
};
use inquire::{Confirm, DateSelect, Select, Text};
use sqlx::FromRow;
use std::{error::Error, fmt};
use termion::{clear, cursor};

#[derive(Debug, FromRow)]
struct Task {
    pub task_name: String,        // *? Is the primary key for sql db.
    pub task_description: String, // *? Text field idk :-)
    pub deadline: NaiveDate,      // *? I can't get how naivedate works but sure.
    pub task_status: Status,      // *? Check the Status enum
}

// *? Debugging purposes.
impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({}, {}, {}, {})",
            self.task_name, self.task_description, self.deadline, self.task_status
        )
    }
}

#[derive(sqlx::Type, Debug)]
#[sqlx(rename_all = "lowercase")]
enum Status {
    Complete,
    InProgress,
    New,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let display = match self {
            Self::New => "New",
            Self::InProgress => "In Progress",
            Self::Complete => "Complete",
        };

        write!(f, "({})", display)
    }
}

enum MenuChoice {
    View,
    Search,
    Add,
    Delete,
    Update,
    Exit,
}

impl fmt::Display for MenuChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let display = match self {
            MenuChoice::View => "View All Tasks",
            MenuChoice::Search => "Search Tasks",
            MenuChoice::Add => "Add Task",
            MenuChoice::Delete => "Delete Task",
            MenuChoice::Update => "Update Task",
            MenuChoice::Exit => "Exit Program",
        };

        write!(f, "{}", display)
    }
}

enum UpdateTaskChoice {
    Description,
    Deadline,
    Status,
}

impl fmt::Display for UpdateTaskChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let display = match self {
            UpdateTaskChoice::Description => "Task Description",
            UpdateTaskChoice::Deadline => "Deadline",
            UpdateTaskChoice::Status => "Task Status",
        };

        write!(f, "{}", display)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // *! IMPORTANT
    let url = "Insert Your Postgres URL here.";
    let pool = sqlx::postgres::PgPool::connect(url).await?;
    // * DB migrations
    sqlx::migrate!("./migrations").run(&pool).await?;
    // * I have no idea any of this works.
    // *! Dreams of Code YT: "SQLx is my favorite PostgreSQL driver to use with Rust." For any troubleshooting.

    // *? Prompts user with menu.
    loop {
        let choices = vec![
            MenuChoice::View,
            MenuChoice::Search,
            MenuChoice::Add,
            MenuChoice::Delete,
            MenuChoice::Update,
            MenuChoice::Exit,
        ];

        // * CHOICE PROMPT
        let input = Select::new("Choose an Option", choices).prompt(); // * Why .expect() not here?

        // * Also if Err in outer match statement below could have been handled with an expect on input, but whatever.
        match input {
            Ok(input) => match input {
                MenuChoice::View => view_all_task(&pool).await?,
                MenuChoice::Search => search_task(&pool).await?,
                MenuChoice::Add => add_task(&pool).await?,
                MenuChoice::Delete => delete_task(&pool).await?,
                MenuChoice::Update => edit_task(&pool).await?,
                MenuChoice::Exit => break,
            },
            Err(_) => println!("An error when choosing an option, please try again."),
        }

        // * CONFIRM PROMPT
        let return_input = Confirm::new("Do you want to return to the main menu?")
            .with_default(true)
            .prompt();

        match return_input {
            Ok(true) => {
                clear_screen();
            }
            Ok(false) => {
                clear_screen();
                break;
            }
            _ => println!("An Error has occurred."),
        }
    }
    Ok(())
}

async fn add_task(pool: &sqlx::PgPool) -> Result<(), Box<dyn Error>> {
    let task_name = Text::new("New Task: ").prompt().expect("Task name error");

    let task_deadline = DateSelect::new("Choose the deadline for your task")
        .prompt()
        .expect("Deadline Error");

    let description = Text::new("Add an description to your task.")
        .prompt()
        .expect("Description Error");

    let status = Status::New;

    let new_task = Task {
        task_name,
        task_description: description,
        deadline: task_deadline,
        task_status: status,
    };

    let query = "INSERT INTO task (task_name, task_description, deadline, task_status) VALUES ($1, $2, $3, $4)";

    // * For any query with lots bind, it is written as this below.
    sqlx::query(query)
        .bind(&new_task.task_name)
        .bind(&new_task.task_description)
        .bind(new_task.deadline)
        .bind(&new_task.task_status)
        .execute(pool)
        .await?;

    Ok(())
}

async fn search_task(pool: &sqlx::PgPool) -> Result<(), Box<dyn Error>> {
    let search_task = Text::new("Search Tasks: ")
        .prompt()
        .expect("Error with input into search.");

    let search_query = format!("%{}%", search_task);

    // * For any query with 0 or 1 bind, it is written as this below.
    // * I'm still new to sql okay >:-(
    let searched_task = sqlx::query_as::<_, Task>("SELECT * FROM task WHERE task_name ILIKE $1 ")
        .bind(search_query)
        .fetch_all(pool)
        .await?;

    if searched_task.last().is_none() {
        println!("No Tasks Found.")
    } else {
        let task_table = create_table(searched_task);
        println!("{task_table}");
    }

    Ok(())
}

async fn view_all_task(pool: &sqlx::PgPool) -> Result<(), Box<dyn Error>> {
    let query = sqlx::query_as::<_, Task>(
        "SELECT task_name, task_description, deadline, task_status FROM task",
    );

    let all_task = query.fetch_all(pool).await?;

    let task_table = create_table(all_task);

    println!("{task_table}");
    Ok(())
}

async fn delete_task(pool: &sqlx::PgPool) -> Result<(), Box<dyn Error>> {
    let search_task = Text::new("Search Tasks: ")
        .prompt()
        .expect("Error with input into search.");

    let search_query = format!("%{}%", search_task);

    let searched_task = sqlx::query_as::<_, Task>("SELECT * FROM task WHERE task_name ILIKE $1 ")
        .bind(search_query)
        .fetch_all(pool)
        .await?;

    let task_name: Vec<&str> = searched_task.iter().map(|t| t.task_name.as_str()).collect();

    // * CHOICE PROMPT
    let delete_choice = Select::new("Choose which task to remove.", task_name)
        .prompt()
        .expect("Failed to get task.");

    let delete_confirm = Confirm::new("Do you want to remove selected task?")
        .prompt()
        .expect("Failed to get delete confirmation.");

    if !delete_confirm {
        println!("Task '{}' was not deleted.", delete_choice);
        return Ok(());
    }

    let formatted_choice = format!("%{}%", &delete_choice);

    sqlx::query_as::<_, Task>("DELETE FROM task WHERE task_name ILIKE $1")
        .bind(formatted_choice)
        .fetch_optional(pool)
        .await?;

    println!("Task '{}' has been deleted.", delete_choice);

    Ok(())
}

async fn edit_task(pool: &sqlx::PgPool) -> Result<(), Box<dyn Error>> {
    let query = sqlx::query_as::<_, Task>(
        "SELECT task_name, task_description, deadline, task_status FROM task",
    );

    let all_tasks = query.fetch_all(pool).await?;

    let tasks_name: Vec<&str> = all_tasks.iter().map(|t| t.task_name.as_str()).collect();

    // * CHOICE PROMPT
    let choice = Select::new("Select Task", tasks_name)
        .prompt()
        .expect("Error with Task Selection");

    let choice_formatted = format!("%{}%", choice);

    let query_selected = sqlx::query_as::<_, Task>("SELECT * FROM task WHERE task_name ILIKE $1 ")
        .bind(choice_formatted)
        .fetch_optional(pool)
        .await?;

    match query_selected {
        Some(selected_task) => loop {
            println!("Update {} Task", selected_task.task_name);
            let options = vec![
                UpdateTaskChoice::Description,
                UpdateTaskChoice::Deadline,
                UpdateTaskChoice::Status,
            ];

            // * CHOICE PROMPT
            let task_choice = Select::new("Select Task", options)
                .prompt()
                .expect("Error with Task Selection");

            match task_choice {
                UpdateTaskChoice::Description => {
                    // * TEXT PROMPT
                    let description_change = Text::new("Change task description to: ")
                        .prompt()
                        .expect("Failed to get new description.");

                    let description_change_query =
                        "UPDATE task SET task_description = $1 WHERE task_name = $2";

                    sqlx::query(description_change_query)
                        .bind(&description_change)
                        .bind(&selected_task.task_name)
                        .execute(pool)
                        .await?;
                    println!("Description has been changed to '{}'.", description_change);
                }

                UpdateTaskChoice::Deadline => {
                    // * CHOICE PROMPT
                    let deadline_change = DateSelect::new("Change deadline to: ")
                        .prompt()
                        .expect("Failed to get new deadline.");

                    let deadline_change_query =
                        "UPDATE task SET deadline = $1 WHERE task_name = $2";

                    sqlx::query(deadline_change_query)
                        .bind(deadline_change)
                        .bind(&selected_task.task_name)
                        .execute(pool)
                        .await?;

                    println!("Deadline has been changed to {}.", deadline_change);
                }

                UpdateTaskChoice::Status => {
                    let status_choices = vec![Status::New, Status::InProgress, Status::Complete];

                    // * CHOICE PROMPT
                    let new_status = Select::new("Update status to: ", status_choices)
                        .prompt()
                        .expect("Failed to get new status.");

                    let status_change_query =
                        "UPDATE task SET task_status = $1 WHERE task_name = $2";

                    sqlx::query(status_change_query)
                        .bind(&new_status)
                        .bind(&selected_task.task_name)
                        .execute(pool)
                        .await?;
                    println!("Status has been changed to '{}'.", new_status);
                }
            }

            // * CONFIRM PROMPT
            let exit_q = Confirm::new("Are you finished updating the task?")
                .prompt()
                .expect("Failed to get Y/n.");

            if exit_q {
                break;
            }
        },
        None => {
            println!("Error with selection.")
        }
    }

    Ok(())
}

fn create_table(tasks: Vec<Task>) -> Table {
    let mut task_table = Table::new();
    // * Table Settings
    task_table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_width(80)
        .set_header(vec![
            Cell::new("Task")
                .fg(Color::Cyan)
                .add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Task Description")
                .fg(Color::DarkCyan)
                .add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Deadline")
                .fg(Color::Magenta)
                .add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Status")
                .fg(Color::DarkMagenta)
                .add_attribute(comfy_table::Attribute::Bold),
        ]);

    // * Adding each tasks to the table in the most super jank way possible
    // * This is redneck Rust.
    for t in tasks.iter() {
        // * Really proud of making the colors of each status type have their own color.
        let status_color = match t.task_status {
            Status::New => Color::Red,
            Status::InProgress => Color::Blue,
            Status::Complete => Color::Green,
        };

        task_table.add_row(vec![
            Cell::new(&t.task_name),
            Cell::new(&t.task_description),
            Cell::new(t.deadline.format("%d-%m-%Y")),
            Cell::new(&t.task_status).fg(status_color),
        ]);
    }

    task_table
}

// * This exist because I didn't want a weird looking println! everywhere
fn clear_screen() {
    println!("{}{}", clear::All, cursor::Goto(1, 1));
}
